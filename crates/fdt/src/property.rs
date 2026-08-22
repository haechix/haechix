use crate::Error;

/// The first address-and-size tuple decoded from an FDT `reg` property.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reg {
    pub address: u64,
    pub size: u64,
}

/// The first address-translation tuple decoded from an FDT `ranges` property.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Range {
    pub child_address: u64,
    pub parent_address: u64,
    pub size: u64,
}

pub fn first_string(value: &[u8]) -> Result<&str, Error> {
    let length = value
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(Error::UnterminatedString)?;

    let string_bytes = value.get(..length).ok_or(Error::UnterminatedString)?;

    core::str::from_utf8(string_bytes).map_err(|_| Error::InvalidUtf8)
}

pub fn stdout_path(value: &[u8]) -> Result<&str, Error> {
    let path_with_options = first_string(value)?;

    let path = path_with_options
        .split_once(':')
        .map_or(path_with_options, |(path, _options)| path);

    let relative = path.strip_prefix('/').ok_or(Error::InvalidNodePath)?;

    if relative.is_empty() || relative.split('/').any(|segment| segment.is_empty()) {
        return Err(Error::InvalidNodePath);
    }

    Ok(path)
}

pub fn u32_value(value: &[u8]) -> Result<u32, Error> {
    let octets: [u8; 4] = value.try_into().map_err(|_| Error::InvalidU32Length {
        actual: value.len(),
    })?;

    Ok(u32::from_be_bytes(octets))
}

pub fn first_reg(value: &[u8], address_cells: u32, size_cells: u32) -> Result<Reg, Error> {
    if !matches!(address_cells, 1 | 2) || !matches!(size_cells, 1 | 2) {
        return Err(Error::UnsupportedCellCount {
            address_cells,
            size_cells,
        });
    }

    let address_cell_count = usize::try_from(address_cells).map_err(|_| Error::IntegerOverflow)?;
    let size_cell_count = usize::try_from(size_cells).map_err(|_| Error::IntegerOverflow)?;

    let address_bytes = address_cell_count
        .checked_mul(4)
        .ok_or(Error::IntegerOverflow)?;
    let size_bytes = size_cell_count
        .checked_mul(4)
        .ok_or(Error::IntegerOverflow)?;
    let required = address_bytes
        .checked_add(size_bytes)
        .ok_or(Error::IntegerOverflow)?;

    if value.len() < required {
        return Err(Error::TruncatedReg {
            required,
            actual: value.len(),
        });
    }

    let address_slice = value.get(..address_bytes).ok_or(Error::TruncatedReg {
        required,
        actual: value.len(),
    })?;

    let size_slice = value
        .get(address_bytes..required)
        .ok_or(Error::TruncatedReg {
            required,
            actual: value.len(),
        })?;

    Ok(Reg {
        address: decode_cells(address_slice)?,
        size: decode_cells(size_slice)?,
    })
}

pub fn first_range(
    value: &[u8],
    child_address_cells: u32,
    parent_address_cells: u32,
    size_cells: u32,
) -> Result<Range, Error> {
    if !matches!(child_address_cells, 1 | 2)
        || !matches!(parent_address_cells, 1 | 2)
        || !matches!(size_cells, 1 | 2)
    {
        return Err(Error::UnsupportedRangeCellCount {
            child_address_cells,
            parent_address_cells,
            size_cells,
        });
    }

    let child_cell_count =
        usize::try_from(child_address_cells).map_err(|_| Error::IntegerOverflow)?;

    let parent_cell_count =
        usize::try_from(parent_address_cells).map_err(|_| Error::IntegerOverflow)?;

    let size_cell_count = usize::try_from(size_cells).map_err(|_| Error::IntegerOverflow)?;

    let child_bytes = child_cell_count
        .checked_mul(4)
        .ok_or(Error::IntegerOverflow)?;

    let parent_bytes = parent_cell_count
        .checked_mul(4)
        .ok_or(Error::IntegerOverflow)?;

    let size_bytes = size_cell_count
        .checked_mul(4)
        .ok_or(Error::IntegerOverflow)?;

    let parent_start = child_bytes;

    let size_start = parent_start
        .checked_add(parent_bytes)
        .ok_or(Error::IntegerOverflow)?;

    let required = size_start
        .checked_add(size_bytes)
        .ok_or(Error::IntegerOverflow)?;

    if value.len() < required {
        return Err(Error::TruncatedRange {
            required,
            actual: value.len(),
        });
    }

    let child = value.get(..parent_start).ok_or(Error::TruncatedRange {
        required,
        actual: value.len(),
    })?;

    let parent = value
        .get(parent_start..size_start)
        .ok_or(Error::TruncatedRange {
            required,
            actual: value.len(),
        })?;

    let size = value
        .get(size_start..required)
        .ok_or(Error::TruncatedRange {
            required,
            actual: value.len(),
        })?;

    Ok(Range {
        child_address: decode_cells(child)?,
        parent_address: decode_cells(parent)?,
        size: decode_cells(size)?,
    })
}

fn decode_cells(value: &[u8]) -> Result<u64, Error> {
    match value.len() {
        4 => Ok(u64::from(u32_value(value)?)),

        8 => {
            let high = value.get(..4).ok_or(Error::InvalidU32Length {
                actual: value.len(),
            })?;

            let low = value.get(4..8).ok_or(Error::InvalidU32Length {
                actual: value.len(),
            })?;

            Ok((u64::from(u32_value(high)?) << 32) | u64::from(u32_value(low)?))
        }

        actual => Err(Error::InvalidU32Length { actual }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_first_string_from_string_list() {
        assert_eq!(
            first_string(b"linux,dummy-virt\0second-value\0"),
            Ok("linux,dummy-virt")
        );
    }

    #[test]
    fn rejects_unterminated_string() {
        assert_eq!(
            first_string(b"missing-null"),
            Err(Error::UnterminatedString)
        );
    }

    #[test]
    fn rejects_invalid_utf8_string() {
        assert_eq!(first_string(&[0xff, 0]), Err(Error::InvalidUtf8));
    }

    #[test]
    fn decodes_big_endian_u32() {
        assert_eq!(u32_value(&[0x12, 0x34, 0x56, 0x78]), Ok(0x1234_5678));
    }

    #[test]
    fn rejects_invalid_u32_length() {
        assert_eq!(
            u32_value(&[0, 0, 0]),
            Err(Error::InvalidU32Length { actual: 3 })
        );
    }

    #[test]
    fn decodes_one_cell_reg_tuple() {
        let value = [0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00];

        assert_eq!(
            first_reg(&value, 1, 1),
            Ok(Reg {
                address: 0x0900_0000,
                size: 0x1000,
            })
        );
    }

    #[test]
    fn decodes_two_cell_reg_tuple() {
        let value = [
            0x00, 0x00, 0x00, 0x01, 0x23, 0x45, 0x67, 0x89, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00,
        ];

        assert_eq!(
            first_reg(&value, 2, 2),
            Ok(Reg {
                address: 0x0000_0001_2345_6789,
                size: 0x1000_0000,
            })
        );
    }

    #[test]
    fn rejects_unsupported_cell_count() {
        assert_eq!(
            first_reg(&[], 3, 1),
            Err(Error::UnsupportedCellCount {
                address_cells: 3,
                size_cells: 1,
            })
        );
    }

    #[test]
    fn rejects_truncated_reg_tuple() {
        let value = [0_u8; 12];

        assert_eq!(
            first_reg(&value, 2, 2),
            Err(Error::TruncatedReg {
                required: 16,
                actual: 12,
            })
        );
    }

    #[test]
    fn reads_absolute_stdout_path_with_suffix() {
        assert_eq!(
            stdout_path(b"/pl011@9000000:115200n8\0"),
            Ok("/pl011@9000000")
        );
    }

    #[test]
    fn rejects_stdout_path_alias() {
        assert_eq!(
            stdout_path(b"serial0:115200n8\0"),
            Err(Error::InvalidNodePath)
        );
    }

    #[test]
    fn rejects_root_stdout_path() {
        assert_eq!(stdout_path(b"/\0"), Err(Error::InvalidNodePath));
    }

    #[test]
    fn rejects_stdout_path_with_empty_segment() {
        assert_eq!(
            stdout_path(b"/soc//pl011@9000000\0"),
            Err(Error::InvalidNodePath)
        );
    }

    #[test]
    fn decodes_simple_bus_range() {
        let value = [
            // Child address: 0x00000000
            0x00, 0x00, 0x00, 0x00, // Parent address: 0x00000010_00000000
            0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, // Size: 0x80000000
            0x80, 0x00, 0x00, 0x00,
        ];

        assert_eq!(
            first_range(&value, 1, 2, 1),
            Ok(Range {
                child_address: 0,
                parent_address: 0x0000_0010_0000_0000,
                size: 0x8000_0000,
            })
        );
    }

    #[test]
    fn rejects_unsupported_range_cell_count() {
        assert_eq!(
            first_range(&[], 3, 2, 1),
            Err(Error::UnsupportedRangeCellCount {
                child_address_cells: 3,
                parent_address_cells: 2,
                size_cells: 1,
            })
        );
    }

    #[test]
    fn rejects_truncated_range() {
        let value = [0_u8; 12];

        assert_eq!(
            first_range(&value, 1, 2, 1),
            Err(Error::TruncatedRange {
                required: 16,
                actual: 12,
            })
        );
    }
}
