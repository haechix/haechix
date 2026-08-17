// Define the boot-time data contract shared by board code and the kernel.
// This crate uses `core` without linking the OS-dependent Rust standard library (`std`).
#![no_std]

/// Normalized hardware information passed from board boot code to the kernel.
///
/// All addresses and sizes must be validated before this structure is created.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootInfo {
    pub memory_start: usize,
    pub memory_size: usize,
    pub uart_base: usize,
    pub interrupt_controller_base: usize,
}
