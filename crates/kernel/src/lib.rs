// Build for bare-metal targets using `core`
// without linking Rust's OS-dependent standard library (`std`).
#![no_std]

use boot_protocol::BootInfo;

/// Starts the platform-independent kernel with validated platform information.
pub fn start(_boot_info: &BootInfo) {}
