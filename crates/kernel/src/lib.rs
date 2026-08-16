// Build for bare-metal targets using `core`
// without linking Rust's OS-dependent standard library (`std`).
#![no_std]

/// Starts the platform-independent kernel after board boot initialization.
pub fn start() {}
