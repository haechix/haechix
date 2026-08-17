use fdt::Fdt;

const DTB_ALIGNMENT: usize = 8;
const MAX_DTB_SIZE: usize = 2 * 1024 * 1024;
const TOTAL_SIZE_OFFSET: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Error {
    NullAddress,
    MisalignedAddress { address: usize },
    InvalidTotalSize { total_size: usize },
    AddressOverflow,
    Parser(fdt::Error),
}

impl From<fdt::Error> for Error {
    fn from(error: fdt::Error) -> Self {
        Self::Parser(error)
    }
}

impl Error {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NullAddress => "null DTB address",

            Self::MisalignedAddress { address } => {
                let _ = address;
                "misaligned DTB address"
            }

            Self::InvalidTotalSize { total_size } => {
                let _ = total_size;
                "invalid DTB total size"
            }

            Self::AddressOverflow => "DTB address overflow",

            Self::Parser(error) => {
                let _ = error;
                "invalid FDT"
            }
        }
    }
}

/// Creates a validated FDT view from the boot-provided physical address.
///
/// # Safety
///
/// The boot environment must guarantee that:
///
/// - `dtb_address` points to a readable FDT header.
/// - The DTB remains readable and immutable for the kernel lifetime.
/// - A valid advertised `totalsize` describes readable contiguous memory.
/// - The MMU is disabled, so the physical address is directly accessible.
pub(crate) unsafe fn from_address(dtb_address: usize) -> Result<Fdt<'static>, Error> {
    if dtb_address == 0 {
        return Err(Error::NullAddress);
    }

    if !dtb_address.is_multiple_of(DTB_ALIGNMENT) {
        return Err(Error::MisalignedAddress {
            address: dtb_address,
        });
    }

    dtb_address
        .checked_add(fdt::HEADER_SIZE)
        .ok_or(Error::AddressOverflow)?;

    let total_size_address = dtb_address
        .checked_add(TOTAL_SIZE_OFFSET)
        .ok_or(Error::AddressOverflow)?;

    let total_size_pointer = total_size_address as *const u32;

    // SAFETY: The caller guarantees that the complete FDT header is readable.
    // read_unaligned creates no reference and the stored value is big-endian.
    let encoded_total_size = unsafe { core::ptr::read_unaligned(total_size_pointer) };

    let total_size =
        usize::try_from(u32::from_be(encoded_total_size)).map_err(|_| Error::AddressOverflow)?;

    if !(fdt::HEADER_SIZE..=MAX_DTB_SIZE).contains(&total_size) {
        return Err(Error::InvalidTotalSize { total_size });
    }

    dtb_address
        .checked_add(total_size)
        .ok_or(Error::AddressOverflow)?;

    // SAFETY: The caller guarantees that the validated range is readable,
    // contiguous, immutable, and remains valid for the kernel lifetime.
    let bytes = unsafe { core::slice::from_raw_parts(dtb_address as *const u8, total_size) };

    Fdt::from_bytes(bytes).map_err(Error::from)
}
