// Build a freestanding QEMU board binary without an operating-system runtime.
#![no_std]
#![no_main]

// Include the AArch64 entry point that runs before Rust code.
core::arch::global_asm!(include_str!("boot.S"));

use arch_aarch64::exception_level::{self, ExceptionLevel};
use drivers::uart::pl011::Pl011;

const QEMU_PL011_BASE_ADDRESS: usize = 0x0900_0000;

fn write_hex_u64(uart: &mut Pl011, value: u64) {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    uart.write_str("0x");

    let mut shift = 60_u32;

    loop {
        let digit = ((value >> shift) & 0xf) as usize;
        uart.write_byte(HEX_DIGITS[digit]);

        if shift == 0 {
            break;
        }

        shift -= 4;
    }
}

// SAFETY: This is the unique exported definition of the boot ABI symbol.
#[unsafe(no_mangle)]
pub extern "C" fn start() {
    // SAFETY: aarch64_normalize_to_el1 enters this function only at EL1h.
    let current_level = unsafe { exception_level::current() };

    // SAFETY: The normalization routine guarantees EL1 execution, and the
    // verified QEMU boot configuration permits access to MPIDR_EL1.
    let mpidr_el1 = unsafe { exception_level::read_mpidr_el1() };

    // SAFETY: The normalization routine guarantees EL1 execution, and the
    // verified QEMU boot configuration permits access to SCTLR_EL1.
    let sctlr_el1 = unsafe { exception_level::read_sctlr_el1() };

    // SAFETY: QEMU virt maps one PL011 register block at 0x0900_0000.
    // Boot currently runs on one virtual CPU, so this is the only owner.
    let mut uart = unsafe { Pl011::new(QEMU_PL011_BASE_ADDRESS) };

    uart.write_str("Haechix M02: QEMU UART OK\n");

    if current_level != ExceptionLevel::El1 {
        uart.write_str("Haechix M03: normalization failed at ");
        uart.write_str(current_level.name());
        uart.write_byte(b'\n');
        return;
    }

    uart.write_str("Haechix M03: EL1 OK\n");

    uart.write_str("CurrentEL=");
    uart.write_str(current_level.name());
    uart.write_byte(b'\n');

    uart.write_str("MPIDR_EL1=");
    write_hex_u64(&mut uart, mpidr_el1);
    uart.write_byte(b'\n');

    uart.write_str("SCTLR_EL1=");
    write_hex_u64(&mut uart, sctlr_el1);
    uart.write_byte(b'\n');

    kernel::start();
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
