//! Reusable AArch64 EL1 exception types and context layout.

use core::arch::asm;

/// Required architectural alignment of an AArch64 exception vector table.
pub const VECTOR_TABLE_ALIGNMENT: usize = 2048;

unsafe extern "C" {
    fn aarch64_exception_vector_table();
}

/// Returns the linked address of the AArch64 exception vector table.
#[must_use]
pub fn vector_table_address() -> usize {
    aarch64_exception_vector_table as *const () as usize
}

/// Installs the linked exception vector table into VBAR_EL1.
///
/// # Safety
///
/// The caller must execute at EL1 with a valid EL1 stack. FP and Advanced
/// SIMD access must already be enabled at EL1 because exception entry saves
/// q0 through q31, FPCR, and FPSR before entering Rust. The linked vector
/// table and exception-entry code must remain resident and executable.
/// Interrupts must remain masked until their handlers are initialized.
/// During M06, only one CPU may execute this installation path.
pub unsafe fn install() {
    let address = vector_table_address();

    debug_assert_eq!(address & (VECTOR_TABLE_ALIGNMENT - 1), 0);

    // SAFETY: The caller guarantees EL1 execution and a permanently resident,
    // 2 KiB-aligned exception vector table.
    unsafe {
        asm!(
            "dsb sy",
            "msr VBAR_EL1, {address}",
            "isb",
            address = in(reg) address,
            options(nostack, preserves_flags),
        );
    }
}

/// Reads the currently installed EL1 exception vector base.
///
/// # Safety
///
/// The caller must execute at an Exception Level that is permitted to read
/// VBAR_EL1. Haechix calls this only after normalization to EL1.
#[must_use]
pub unsafe fn read_vbar_el1() -> u64 {
    let value: u64;

    // SAFETY: The caller guarantees that reading VBAR_EL1 is permitted.
    unsafe {
        asm!(
            "mrs {value}, VBAR_EL1",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Number of AArch64 general-purpose registers captured on exception entry.
pub const GENERAL_REGISTER_COUNT: usize = 31;

/// Size of [`ExceptionContext`] shared by Rust and exception-entry assembly.
pub const EXCEPTION_CONTEXT_SIZE: usize = 304;

/// Required alignment of [`ExceptionContext`].
pub const EXCEPTION_CONTEXT_ALIGNMENT: usize = 16;

const ESR_EXCEPTION_CLASS_SHIFT: u64 = 26;
const ESR_EXCEPTION_CLASS_MASK: u64 = 0x3f;
const ESR_INSTRUCTION_LENGTH_BIT: u64 = 1 << 25;
const ESR_ISS_MASK: u64 = (1 << 25) - 1;

/// Identifies one of the sixteen AArch64 exception-vector slots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum VectorId {
    CurrentElSp0Synchronous = 0,
    CurrentElSp0Irq = 1,
    CurrentElSp0Fiq = 2,
    CurrentElSp0SError = 3,
    CurrentElSpxSynchronous = 4,
    CurrentElSpxIrq = 5,
    CurrentElSpxFiq = 6,
    CurrentElSpxSError = 7,
    LowerElAarch64Synchronous = 8,
    LowerElAarch64Irq = 9,
    LowerElAarch64Fiq = 10,
    LowerElAarch64SError = 11,
    LowerElAarch32Synchronous = 12,
    LowerElAarch32Irq = 13,
    LowerElAarch32Fiq = 14,
    LowerElAarch32SError = 15,
}

impl VectorId {
    /// Converts the raw vector identifier written by assembly.
    #[must_use]
    pub const fn from_raw(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::CurrentElSp0Synchronous),
            1 => Some(Self::CurrentElSp0Irq),
            2 => Some(Self::CurrentElSp0Fiq),
            3 => Some(Self::CurrentElSp0SError),
            4 => Some(Self::CurrentElSpxSynchronous),
            5 => Some(Self::CurrentElSpxIrq),
            6 => Some(Self::CurrentElSpxFiq),
            7 => Some(Self::CurrentElSpxSError),
            8 => Some(Self::LowerElAarch64Synchronous),
            9 => Some(Self::LowerElAarch64Irq),
            10 => Some(Self::LowerElAarch64Fiq),
            11 => Some(Self::LowerElAarch64SError),
            12 => Some(Self::LowerElAarch32Synchronous),
            13 => Some(Self::LowerElAarch32Irq),
            14 => Some(Self::LowerElAarch32Fiq),
            15 => Some(Self::LowerElAarch32SError),
            _ => None,
        }
    }

    /// Returns the stable diagnostic name of the vector slot.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CurrentElSp0Synchronous => "current-el-sp0-sync",
            Self::CurrentElSp0Irq => "current-el-sp0-irq",
            Self::CurrentElSp0Fiq => "current-el-sp0-fiq",
            Self::CurrentElSp0SError => "current-el-sp0-serror",
            Self::CurrentElSpxSynchronous => "current-el-spx-sync",
            Self::CurrentElSpxIrq => "current-el-spx-irq",
            Self::CurrentElSpxFiq => "current-el-spx-fiq",
            Self::CurrentElSpxSError => "current-el-spx-serror",
            Self::LowerElAarch64Synchronous => "lower-el-aarch64-sync",
            Self::LowerElAarch64Irq => "lower-el-aarch64-irq",
            Self::LowerElAarch64Fiq => "lower-el-aarch64-fiq",
            Self::LowerElAarch64SError => "lower-el-aarch64-serror",
            Self::LowerElAarch32Synchronous => "lower-el-aarch32-sync",
            Self::LowerElAarch32Irq => "lower-el-aarch32-irq",
            Self::LowerElAarch32Fiq => "lower-el-aarch32-fiq",
            Self::LowerElAarch32SError => "lower-el-aarch32-serror",
        }
    }
}

/// CPU state saved by the AArch64 exception-entry assembly.
///
/// The field order and offsets are part of the assembly/Rust ABI.
/// Any layout change must be reflected in `exception.S`.
#[repr(C, align(16))]
pub struct ExceptionContext {
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,
    pub x30: u64,
    pub sp_at_entry: u64,
    pub elr_el1: u64,
    pub spsr_el1: u64,
    pub esr_el1: u64,
    pub far_el1: u64,
    pub vector_id: u64,
    pub reserved: u64,
}

impl ExceptionContext {
    /// Returns the architectural vector identifier.
    #[must_use]
    pub const fn vector(&self) -> Option<VectorId> {
        VectorId::from_raw(self.vector_id)
    }

    /// Returns the six-bit ESR_EL1 exception class.
    #[must_use]
    pub const fn exception_class(&self) -> u8 {
        ((self.esr_el1 >> ESR_EXCEPTION_CLASS_SHIFT) & ESR_EXCEPTION_CLASS_MASK) as u8
    }

    /// Reports whether ESR_EL1 describes a 32-bit trapped instruction.
    #[must_use]
    pub const fn instruction_length_is_32_bit(&self) -> bool {
        self.esr_el1 & ESR_INSTRUCTION_LENGTH_BIT != 0
    }

    /// Returns the ESR_EL1 instruction-specific syndrome.
    #[must_use]
    pub const fn instruction_specific_syndrome(&self) -> u32 {
        (self.esr_el1 & ESR_ISS_MASK) as u32
    }
}

const _: () = {
    assert!(core::mem::size_of::<ExceptionContext>() == EXCEPTION_CONTEXT_SIZE);
    assert!(core::mem::align_of::<ExceptionContext>() == EXCEPTION_CONTEXT_ALIGNMENT);

    assert!(core::mem::offset_of!(ExceptionContext, x0) == 0);
    assert!(core::mem::offset_of!(ExceptionContext, x30) == 240);
    assert!(core::mem::offset_of!(ExceptionContext, sp_at_entry) == 248);
    assert!(core::mem::offset_of!(ExceptionContext, elr_el1) == 256);
    assert!(core::mem::offset_of!(ExceptionContext, spsr_el1) == 264);
    assert!(core::mem::offset_of!(ExceptionContext, esr_el1) == 272);
    assert!(core::mem::offset_of!(ExceptionContext, far_el1) == 280);
    assert!(core::mem::offset_of!(ExceptionContext, vector_id) == 288);
    assert!(core::mem::offset_of!(ExceptionContext, reserved) == 296);
};
