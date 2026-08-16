// Build a freestanding QEMU board binary without an operating-system runtime.
#![no_std]
#![no_main]

// Include the AArch64 entry point that runs before Rust code.
core::arch::global_asm!(include_str!("boot.S"));

// SAFETY: This is the unique exported definition of the boot ABI symbol.
#[unsafe(no_mangle)]
pub extern "C" fn start() {
    kernel::start();
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
