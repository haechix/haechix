use crate::error::Error;

pub(crate) const FDT_HEADER_SIZE: usize = 40;
pub(crate) const FDT_MAGIC: u32 = 0xd00d_feed;
pub(crate) const MIN_SUPPORTED_VERSION: u32 = 17;
pub(crate) const MAX_LAST_COMPATIBLE_VERSION: u32 = 17;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Header {
    pub(crate) total_size: usize,
    pub(crate) structure_offset: usize,
    pub(crate) strings_offset: usize,
    pub(crate) memory_reservation_offset: usize,
    pub(crate) version: u32,
    pub(crate) last_compatible_version: u32,
    pub(crate) boot_cpu_id: u32,
    pub(crate) strings_size: usize,
    pub(crate) structure_size: usize,
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let end = offset.checked_add(4).ok_or(Error::IntegerOverflow)?;

    let field = bytes.get(offset..end).ok_or(Error::HeaderTooSmall {
        actual: bytes.len(),
    })?;

    let octets: [u8; 4] = field.try_into().map_err(|_| Error::HeaderTooSmall {
        actual: bytes.len(),
    })?;

    Ok(u32::from_be_bytes(octets))
}

impl Header {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < FDT_HEADER_SIZE {
            return Err(Error::HeaderTooSmall {
                actual: bytes.len(),
            });
        }

        let magic = read_be_u32(bytes, 0)?;

        if magic != FDT_MAGIC {
            return Err(Error::InvalidMagic { actual: magic });
        }

        let total_size =
            usize::try_from(read_be_u32(bytes, 4)?).map_err(|_| Error::IntegerOverflow)?;
        let structure_offset =
            usize::try_from(read_be_u32(bytes, 8)?).map_err(|_| Error::IntegerOverflow)?;
        let strings_offset =
            usize::try_from(read_be_u32(bytes, 12)?).map_err(|_| Error::IntegerOverflow)?;
        let memory_reservation_offset =
            usize::try_from(read_be_u32(bytes, 16)?).map_err(|_| Error::IntegerOverflow)?;
        let version = read_be_u32(bytes, 20)?;
        let last_compatible_version = read_be_u32(bytes, 24)?;
        let boot_cpu_id = read_be_u32(bytes, 28)?;
        let strings_size =
            usize::try_from(read_be_u32(bytes, 32)?).map_err(|_| Error::IntegerOverflow)?;
        let structure_size =
            usize::try_from(read_be_u32(bytes, 36)?).map_err(|_| Error::IntegerOverflow)?;

        if total_size < FDT_HEADER_SIZE || total_size > bytes.len() {
            return Err(Error::InvalidTotalSize {
                total_size,
                available: bytes.len(),
            });
        }

        if version < MIN_SUPPORTED_VERSION || last_compatible_version > MAX_LAST_COMPATIBLE_VERSION
        {
            return Err(Error::UnsupportedVersion {
                version,
                last_compatible_version,
            });
        }

        if structure_offset % 4 != 0 {
            return Err(Error::MisalignedStructureBlock);
        }

        if memory_reservation_offset % 8 != 0 {
            return Err(Error::MisalignedMemoryReservationBlock);
        }

        let structure_end = structure_offset
            .checked_add(structure_size)
            .ok_or(Error::IntegerOverflow)?;

        if structure_offset < FDT_HEADER_SIZE || structure_end > total_size {
            return Err(Error::StructureBlockOutOfBounds);
        }

        let strings_end = strings_offset
            .checked_add(strings_size)
            .ok_or(Error::IntegerOverflow)?;

        if strings_offset < FDT_HEADER_SIZE || strings_end > total_size {
            return Err(Error::StringsBlockOutOfBounds);
        }

        let reservation_entry_end = memory_reservation_offset
            .checked_add(16)
            .ok_or(Error::IntegerOverflow)?;

        if memory_reservation_offset < FDT_HEADER_SIZE || reservation_entry_end > total_size {
            return Err(Error::MemoryReservationBlockOutOfBounds);
        }

        if reservation_entry_end > structure_offset || structure_end > strings_offset {
            return Err(Error::OverlappingBlocks);
        }

        Ok(Self {
            total_size,
            structure_offset,
            strings_offset,
            memory_reservation_offset,
            version,
            last_compatible_version,
            boot_cpu_id,
            strings_size,
            structure_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BLOB_SIZE: usize = 64;

    fn write_be_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn valid_blob() -> [u8; TEST_BLOB_SIZE] {
        let mut bytes = [0_u8; TEST_BLOB_SIZE];

        write_be_u32(&mut bytes, 0, FDT_MAGIC);
        write_be_u32(&mut bytes, 4, TEST_BLOB_SIZE as u32);
        write_be_u32(&mut bytes, 8, 56);
        write_be_u32(&mut bytes, 12, 60);
        write_be_u32(&mut bytes, 16, 40);
        write_be_u32(&mut bytes, 20, 17);
        write_be_u32(&mut bytes, 24, 16);
        write_be_u32(&mut bytes, 28, 0);
        write_be_u32(&mut bytes, 32, 4);
        write_be_u32(&mut bytes, 36, 4);

        bytes
    }

    #[test]
    fn parses_valid_header() {
        let bytes = valid_blob();
        let header = Header::parse(&bytes).unwrap();

        assert_eq!(header.total_size, 64);
        assert_eq!(header.structure_offset, 56);
        assert_eq!(header.strings_offset, 60);
        assert_eq!(header.memory_reservation_offset, 40);
        assert_eq!(header.version, 17);
        assert_eq!(header.last_compatible_version, 16);
        assert_eq!(header.boot_cpu_id, 0);
        assert_eq!(header.strings_size, 4);
        assert_eq!(header.structure_size, 4);
    }

    #[test]
    fn rejects_truncated_header() {
        let bytes = valid_blob();

        assert_eq!(
            Header::parse(&bytes[..FDT_HEADER_SIZE - 1]),
            Err(Error::HeaderTooSmall {
                actual: FDT_HEADER_SIZE - 1,
            })
        );
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 0, 0);

        assert_eq!(
            Header::parse(&bytes),
            Err(Error::InvalidMagic { actual: 0 })
        );
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 20, 16);

        assert_eq!(
            Header::parse(&bytes),
            Err(Error::UnsupportedVersion {
                version: 16,
                last_compatible_version: 16,
            })
        );
    }

    #[test]
    fn rejects_out_of_bounds_structure_block() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 8, 60);
        write_be_u32(&mut bytes, 36, 8);

        assert_eq!(Header::parse(&bytes), Err(Error::StructureBlockOutOfBounds));
    }

    #[test]
    fn rejects_misaligned_structure_block() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 8, 58);

        assert_eq!(Header::parse(&bytes), Err(Error::MisalignedStructureBlock));
    }

    #[test]
    fn rejects_misaligned_memory_reservation_block() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 16, 44);

        assert_eq!(
            Header::parse(&bytes),
            Err(Error::MisalignedMemoryReservationBlock)
        );
    }

    #[test]
    fn rejects_overlapping_structure_and_strings_blocks() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 36, 8);

        assert_eq!(Header::parse(&bytes), Err(Error::OverlappingBlocks));
    }

    #[test]
    fn rejects_out_of_bounds_strings_block() {
        let mut bytes = valid_blob();
        write_be_u32(&mut bytes, 12, 64);

        assert_eq!(Header::parse(&bytes), Err(Error::StringsBlockOutOfBounds));
    }
}
