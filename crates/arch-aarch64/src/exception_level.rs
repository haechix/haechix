//! AArch64 Exception Level identification and EL1 register access.

use core::arch::asm;

/// An AArch64 Exception Level encoding.
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ExceptionLevel {
    El0 = 0,
    El1 = 1,
    El2 = 2,
    El3 = 3,
}

impl ExceptionLevel {
    /// Decodes the two-bit Exception Level encoding.
    pub const fn from_encoding(encoding: u8) -> Option<Self> {
        match encoding {
            0 => Some(Self::El0),
            1 => Some(Self::El1),
            2 => Some(Self::El2),
            3 => Some(Self::El3),
            _ => None,
        }
    }

    /// Returns the architectural name of this Exception Level.
    pub const fn name(self) -> &'static str {
        match self {
            Self::El0 => "EL0",
            Self::El1 => "EL1",
            Self::El2 => "EL2",
            Self::El3 => "EL3",
        }
    }
}

/// Reads and decodes the current AArch64 Exception Level.
///
/// # Safety
///
/// The caller must guarantee that execution is currently at EL1 or higher.
/// Reading `CurrentEL` from EL0 is architecturally undefined.
#[inline]
pub unsafe fn current() -> ExceptionLevel {
    let raw_value: u64;

    // SAFETY: The caller guarantees that CurrentEL is accessible from the
    // active Exception Level. This instruction does not access memory.
    unsafe {
        asm!(
            "mrs {raw_value}, CurrentEL",
            raw_value = out(reg) raw_value,
            options(nomem, nostack, preserves_flags),
        );
    }

    let encoding = ((raw_value >> 2) & 0b11) as u8;

    match ExceptionLevel::from_encoding(encoding) {
        Some(level) => level,
        None => unreachable!(),
    }
}

/// Reads the Multiprocessor Affinity Register visible at EL1.
///
/// # Safety
///
/// The caller must guarantee that `MPIDR_EL1` is accessible and is not
/// configured to trap to a higher Exception Level.
#[inline]
pub unsafe fn read_mpidr_el1() -> u64 {
    let value: u64;

    // SAFETY: The caller guarantees access to MPIDR_EL1. The instruction
    // only reads a system register and does not access memory.
    unsafe {
        asm!(
            "mrs {value}, MPIDR_EL1",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Reads the System Control Register visible at EL1.
///
/// # Safety
///
/// The caller must guarantee that `SCTLR_EL1` is accessible and is not
/// configured to trap to a higher Exception Level.
#[inline]
pub unsafe fn read_sctlr_el1() -> u64 {
    let value: u64;

    // SAFETY: The caller guarantees access to SCTLR_EL1. The instruction
    // only reads a system register and does not access memory.
    unsafe {
        asm!(
            "mrs {value}, SCTLR_EL1",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}
