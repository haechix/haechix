// Build reusable AArch64 support for bare-metal targets.
// This crate uses `core` without linking the OS-dependent Rust standard library (`std`).
#![no_std]

// Include the stackless Exception Level normalization routine.
core::arch::global_asm!(include_str!("exception_level.S"));

// Include the reusable EL1 exception vector and context entry routine.
core::arch::global_asm!(include_str!("exception.S"));

pub mod exception;
pub mod exception_level;
