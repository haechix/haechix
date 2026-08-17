// Parse Flattened Device Tree blobs without dynamic allocation.
// This crate uses `core` without linking the OS-dependent Rust standard library (`std`).
#![no_std]

mod error;
mod header;
mod property;
mod structure;

pub use error::Error;
pub use property::{Reg, first_reg, first_string, stdout_path, u32_value};
pub use structure::{Token, Tokens};

/// Size in bytes of an FDT version 17 header.
pub const HEADER_SIZE: usize = header::FDT_HEADER_SIZE;

use header::Header;

/// A validated borrowed view of a Flattened Device Tree blob.
#[derive(Clone, Copy, Debug)]
pub struct Fdt<'a> {
    bytes: &'a [u8],
    structure_block: &'a [u8],
    strings_block: &'a [u8],
    header: Header,
}

impl<'a> Fdt<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        let header = Header::parse(bytes)?;

        let blob = bytes
            .get(..header.total_size)
            .ok_or(Error::InvalidTotalSize {
                total_size: header.total_size,
                available: bytes.len(),
            })?;

        let structure_end = header
            .structure_offset
            .checked_add(header.structure_size)
            .ok_or(Error::IntegerOverflow)?;

        let structure_block = blob
            .get(header.structure_offset..structure_end)
            .ok_or(Error::StructureBlockOutOfBounds)?;

        let strings_end = header
            .strings_offset
            .checked_add(header.strings_size)
            .ok_or(Error::IntegerOverflow)?;

        let strings_block = blob
            .get(header.strings_offset..strings_end)
            .ok_or(Error::StringsBlockOutOfBounds)?;

        let fdt = Self {
            bytes: blob,
            structure_block,
            strings_block,
            header,
        };

        for token in fdt.tokens() {
            token?;
        }

        Ok(fdt)
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn total_size(&self) -> usize {
        self.header.total_size
    }

    pub fn structure_block(&self) -> &'a [u8] {
        self.structure_block
    }

    pub fn strings_block(&self) -> &'a [u8] {
        self.strings_block
    }

    pub fn memory_reservation_offset(&self) -> usize {
        self.header.memory_reservation_offset
    }

    pub fn version(&self) -> u32 {
        self.header.version
    }

    pub fn last_compatible_version(&self) -> u32 {
        self.header.last_compatible_version
    }

    pub fn boot_cpu_id(&self) -> u32 {
        self.header.boot_cpu_id
    }

    pub fn tokens(&self) -> Tokens<'a> {
        Tokens::new(self.structure_block, self.strings_block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BLOB_SIZE: usize = 72;
    const FDT_BEGIN_NODE: u32 = 1;
    const FDT_END_NODE: u32 = 2;
    const FDT_END: u32 = 9;

    fn write_be_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn valid_minimal_fdt() -> [u8; TEST_BLOB_SIZE] {
        let mut bytes = [0_u8; TEST_BLOB_SIZE];

        write_be_u32(&mut bytes, 0, header::FDT_MAGIC);
        write_be_u32(&mut bytes, 4, TEST_BLOB_SIZE as u32);
        write_be_u32(&mut bytes, 8, 56);
        write_be_u32(&mut bytes, 12, 72);
        write_be_u32(&mut bytes, 16, 40);
        write_be_u32(&mut bytes, 20, 17);
        write_be_u32(&mut bytes, 24, 16);
        write_be_u32(&mut bytes, 28, 0);
        write_be_u32(&mut bytes, 32, 0);
        write_be_u32(&mut bytes, 36, 16);

        write_be_u32(&mut bytes, 56, FDT_BEGIN_NODE);
        write_be_u32(&mut bytes, 64, FDT_END_NODE);
        write_be_u32(&mut bytes, 68, FDT_END);

        bytes
    }

    #[test]
    fn parses_valid_minimal_fdt() {
        let bytes = valid_minimal_fdt();
        let fdt = Fdt::from_bytes(&bytes).unwrap();
        let mut tokens = fdt.tokens();

        assert_eq!(fdt.total_size(), TEST_BLOB_SIZE);
        assert_eq!(tokens.next(), Some(Ok(Token::BeginNode { name: "" })));
        assert_eq!(tokens.next(), Some(Ok(Token::EndNode)));
        assert_eq!(tokens.next(), Some(Ok(Token::End)));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn rejects_unknown_token_during_construction() {
        let mut bytes = valid_minimal_fdt();
        write_be_u32(&mut bytes, 56, 0xff);

        assert_eq!(
            Fdt::from_bytes(&bytes).unwrap_err(),
            Error::UnknownStructureToken { value: 0xff }
        );
    }
}
