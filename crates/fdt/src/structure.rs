use crate::Error;

const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

/// A validated token borrowed from an FDT structure block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Token<'a> {
    BeginNode { name: &'a str },
    EndNode,
    Property { name: &'a str, value: &'a [u8] },
    Nop,
    End,
}

/// An allocation-free iterator over an FDT structure block.
#[derive(Debug)]
pub struct Tokens<'a> {
    structure_block: &'a [u8],
    strings_block: &'a [u8],
    cursor: usize,
    depth: usize,
    finished: bool,
}

impl<'a> Tokens<'a> {
    pub(crate) fn new(structure_block: &'a [u8], strings_block: &'a [u8]) -> Self {
        Self {
            structure_block,
            strings_block,
            cursor: 0,
            depth: 0,
            finished: false,
        }
    }
}

fn read_be_u32(bytes: &[u8], offset: usize, truncated_error: Error) -> Result<u32, Error> {
    let end = offset.checked_add(4).ok_or(Error::IntegerOverflow)?;

    let field = bytes.get(offset..end).ok_or(truncated_error)?;

    let octets: [u8; 4] = field.try_into().map_err(|_| truncated_error)?;

    Ok(u32::from_be_bytes(octets))
}

fn align_to_four(value: usize) -> Result<usize, Error> {
    value
        .checked_add(3)
        .map(|aligned| aligned & !3)
        .ok_or(Error::IntegerOverflow)
}

fn read_node_name(bytes: &[u8], offset: usize) -> Result<(&str, usize), Error> {
    let remaining = bytes.get(offset..).ok_or(Error::StructureBlockTruncated)?;

    let name_length = remaining
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(Error::UnterminatedNodeName)?;

    let name_bytes = remaining
        .get(..name_length)
        .ok_or(Error::StructureBlockTruncated)?;

    let name = core::str::from_utf8(name_bytes).map_err(|_| Error::InvalidUtf8)?;

    let consumed = name_length.checked_add(1).ok_or(Error::IntegerOverflow)?;

    let unaligned_next = offset.checked_add(consumed).ok_or(Error::IntegerOverflow)?;

    let next = align_to_four(unaligned_next)?;

    if next > bytes.len() {
        return Err(Error::StructureBlockTruncated);
    }

    Ok((name, next))
}

fn read_property_name(strings: &[u8], offset: usize) -> Result<&str, Error> {
    let remaining = strings
        .get(offset..)
        .ok_or(Error::PropertyNameOutOfBounds { offset })?;

    let name_length = remaining
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(Error::UnterminatedPropertyName)?;

    let name_bytes = remaining
        .get(..name_length)
        .ok_or(Error::UnterminatedPropertyName)?;

    core::str::from_utf8(name_bytes).map_err(|_| Error::InvalidUtf8)
}

impl<'a> Tokens<'a> {
    fn parse_next(&mut self) -> Result<Token<'a>, Error> {
        if self.cursor >= self.structure_block.len() {
            return Err(Error::MissingEndToken);
        }

        let token = read_be_u32(
            self.structure_block,
            self.cursor,
            Error::StructureBlockTruncated,
        )?;

        self.cursor = self.cursor.checked_add(4).ok_or(Error::IntegerOverflow)?;

        match token {
            FDT_BEGIN_NODE => {
                let (name, next) = read_node_name(self.structure_block, self.cursor)?;

                self.cursor = next;
                self.depth = self.depth.checked_add(1).ok_or(Error::IntegerOverflow)?;

                Ok(Token::BeginNode { name })
            }

            FDT_END_NODE => {
                self.depth = self.depth.checked_sub(1).ok_or(Error::UnexpectedEndNode)?;

                Ok(Token::EndNode)
            }

            FDT_PROP => {
                if self.depth == 0 {
                    return Err(Error::PropertyOutsideNode);
                }

                let value_length = usize::try_from(read_be_u32(
                    self.structure_block,
                    self.cursor,
                    Error::TruncatedProperty,
                )?)
                .map_err(|_| Error::IntegerOverflow)?;

                let name_offset_position =
                    self.cursor.checked_add(4).ok_or(Error::IntegerOverflow)?;

                let name_offset = usize::try_from(read_be_u32(
                    self.structure_block,
                    name_offset_position,
                    Error::TruncatedProperty,
                )?)
                .map_err(|_| Error::IntegerOverflow)?;

                let value_start = self.cursor.checked_add(8).ok_or(Error::IntegerOverflow)?;

                let value_end = value_start
                    .checked_add(value_length)
                    .ok_or(Error::IntegerOverflow)?;

                let next = align_to_four(value_end)?;

                if next > self.structure_block.len() {
                    return Err(Error::TruncatedProperty);
                }

                let value = self
                    .structure_block
                    .get(value_start..value_end)
                    .ok_or(Error::TruncatedProperty)?;

                let name = read_property_name(self.strings_block, name_offset)?;

                self.cursor = next;

                Ok(Token::Property { name, value })
            }

            FDT_NOP => Ok(Token::Nop),

            FDT_END => {
                if self.depth != 0 {
                    return Err(Error::UnclosedNodes { depth: self.depth });
                }

                if self.cursor != self.structure_block.len() {
                    return Err(Error::TrailingStructureData);
                }

                self.finished = true;

                Ok(Token::End)
            }

            value => Err(Error::UnknownStructureToken { value }),
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.parse_next() {
            Ok(token) => Some(Ok(token)),
            Err(error) => {
                self.finished = true;
                Some(Err(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROPERTY_NAME: &[u8] = b"compatible\0";

    fn write_be_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn valid_structure() -> [u8; 32] {
        let mut bytes = [0_u8; 32];

        write_be_u32(&mut bytes, 0, FDT_BEGIN_NODE);
        write_be_u32(&mut bytes, 8, FDT_PROP);
        write_be_u32(&mut bytes, 12, 4);
        write_be_u32(&mut bytes, 16, 0);
        bytes[20..24].copy_from_slice(b"test");
        write_be_u32(&mut bytes, 24, FDT_END_NODE);
        write_be_u32(&mut bytes, 28, FDT_END);

        bytes
    }

    #[test]
    fn iterates_valid_structure_tokens() {
        let structure = valid_structure();
        let mut tokens = Tokens::new(&structure, PROPERTY_NAME);

        assert_eq!(tokens.next(), Some(Ok(Token::BeginNode { name: "" })));

        assert_eq!(
            tokens.next(),
            Some(Ok(Token::Property {
                name: "compatible",
                value: b"test",
            }))
        );

        assert_eq!(tokens.next(), Some(Ok(Token::EndNode)));
        assert_eq!(tokens.next(), Some(Ok(Token::End)));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn rejects_unknown_token() {
        let structure = [0, 0, 0, 0xff];
        let mut tokens = Tokens::new(&structure, &[]);

        assert_eq!(
            tokens.next(),
            Some(Err(Error::UnknownStructureToken { value: 0xff }))
        );
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn rejects_unterminated_node_name() {
        let structure = [0, 0, 0, 1, b'n', b'o', b'd', b'e'];
        let mut tokens = Tokens::new(&structure, &[]);

        assert_eq!(tokens.next(), Some(Err(Error::UnterminatedNodeName)));
    }

    #[test]
    fn rejects_property_name_out_of_bounds() {
        let mut structure = valid_structure();
        write_be_u32(&mut structure, 16, 100);

        let mut tokens = Tokens::new(&structure, PROPERTY_NAME);

        assert_eq!(tokens.next(), Some(Ok(Token::BeginNode { name: "" })));

        assert_eq!(
            tokens.next(),
            Some(Err(Error::PropertyNameOutOfBounds { offset: 100 }))
        );
    }

    #[test]
    fn rejects_missing_end_token() {
        let structure = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];

        let mut tokens = Tokens::new(&structure, &[]);

        assert_eq!(tokens.next(), Some(Ok(Token::BeginNode { name: "" })));
        assert_eq!(tokens.next(), Some(Ok(Token::EndNode)));
        assert_eq!(tokens.next(), Some(Err(Error::MissingEndToken)));
    }

    #[test]
    fn rejects_unclosed_node() {
        let structure = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 9];

        let mut tokens = Tokens::new(&structure, &[]);

        assert_eq!(tokens.next(), Some(Ok(Token::BeginNode { name: "" })));

        assert_eq!(tokens.next(), Some(Err(Error::UnclosedNodes { depth: 1 })));
    }
    #[test]
    fn rejects_property_outside_node() {
        let structure = [0, 0, 0, 3];
        let mut tokens = Tokens::new(&structure, &[]);

        assert_eq!(tokens.next(), Some(Err(Error::PropertyOutsideNode)));
    }
}
