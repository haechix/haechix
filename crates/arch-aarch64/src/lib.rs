// Build reusable AArch64 support for bare-metal targets.
// This crate uses `core` without linking the OS-dependent Rust standard library (`std`).
#![no_std]

// Include the stackless Exception Level normalization routine.
core::arch::global_asm!(include_str!("exception_level.S"));

pub mod exception_level;
