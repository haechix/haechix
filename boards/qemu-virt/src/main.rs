// Build a freestanding QEMU board binary without an operating-system runtime.
#![no_std]
#![no_main]

// Include the AArch64 entry point that runs before Rust code.
core::arch::global_asm!(include_str!("boot.S"));

use drivers::uart::pl011::Pl011;

const QEMU_PL011_BASE_ADDRESS: usize = 0x0900_0000;

// SAFETY: This is the unique exported definition of the boot ABI symbol.
#[unsafe(no_mangle)]
pub extern "C" fn start() {
    // SAFETY: QEMU virt maps one PL011 register block at 0x0900_0000.
    // Boot currently runs on one virtual CPU, so this is the only owner.
    let mut uart = unsafe { Pl011::new(QEMU_PL011_BASE_ADDRESS) };

    uart.write_str("Haechix M02: QEMU UART OK\n");
    kernel::start();
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
