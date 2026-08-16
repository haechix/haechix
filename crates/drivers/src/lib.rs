// Build reusable device drivers for bare-metal targets.
// This crate uses `core` without linking the OS-dependent Rust standard library (`std`).
#![no_std]
pub mod uart;
