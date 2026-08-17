// Build a freestanding QEMU board binary without an operating-system runtime.
#![no_std]
#![no_main]

mod dtb;
mod platform;

// Include the AArch64 entry point that runs before Rust code.
core::arch::global_asm!(include_str!("boot.S"));

use arch_aarch64::exception_level::{self, ExceptionLevel};
use boot_protocol::BootInfo;
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
pub extern "C" fn start(dtb_address: usize) {
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

    // SAFETY: QEMU's AArch64 raw-kernel boot protocol provides a readable,
    // 8-byte-aligned DTB in x0 that remains valid during kernel execution.
    let fdt = match unsafe { dtb::from_address(dtb_address) } {
        Ok(fdt) => fdt,

        Err(error) => {
            uart.write_str("Haechix M04: DTB error: ");
            uart.write_str(error.message());
            uart.write_byte(b'\n');
            return;
        }
    };

    let platform_info = match platform::discover(&fdt) {
        Ok(platform_info) => platform_info,

        Err(error) => {
            uart.write_str("Haechix M04: DTB error: ");
            uart.write_str(error.message());
            uart.write_byte(b'\n');
            return;
        }
    };
    let parsed_uart_base = match usize::try_from(platform_info.uart.address) {
        Ok(address) => address,

        Err(_) => {
            uart.write_str("Haechix M04: DTB error: console address out of range\n");
            return;
        }
    };

    if parsed_uart_base != QEMU_PL011_BASE_ADDRESS {
        uart.write_str("Haechix M04: DTB error: console address changed during bootstrap\n");
        return;
    }

    let memory_end = match platform_info
        .memory
        .address
        .checked_add(platform_info.memory.size)
    {
        Some(end) => end,

        None => {
            uart.write_str("Haechix M04: DTB error: memory range overflow\n");
            return;
        }
    };

    let (memory_start, memory_size, interrupt_controller_base) = match (
        usize::try_from(platform_info.memory.address),
        usize::try_from(platform_info.memory.size),
        usize::try_from(platform_info.interrupt_controller.address),
    ) {
        (Ok(memory_start), Ok(memory_size), Ok(interrupt_controller_base)) => {
            (memory_start, memory_size, interrupt_controller_base)
        }

        _ => {
            uart.write_str("Haechix M04: DTB error: platform value out of usize range\n");
            return;
        }
    };

    if memory_size == 0 || memory_start.checked_add(memory_size).is_none() {
        uart.write_str("Haechix M04: DTB error: invalid BootInfo memory range\n");
        return;
    }

    let boot_info = BootInfo {
        memory_start,
        memory_size,
        uart_base: parsed_uart_base,
        interrupt_controller_base,
    };

    // End ownership of the bootstrap UART before constructing the
    // DTB-discovered PL011 for the same MMIO register block.
    uart.release();

    // SAFETY: The DTB identifies this address as a PL011 register block.
    // The bootstrap owner was released above, and boot uses one CPU.
    let mut uart = unsafe { Pl011::new(parsed_uart_base) };

    uart.write_str("Haechix M04: DTB OK\n");

    uart.write_str("compatible=");
    uart.write_str(platform_info.compatible);
    uart.write_byte(b'\n');

    uart.write_str("memory=");
    write_hex_u64(&mut uart, platform_info.memory.address);
    uart.write_str("..");
    write_hex_u64(&mut uart, memory_end);
    uart.write_byte(b'\n');

    uart.write_str("console=pl011@");
    write_hex_u64(&mut uart, platform_info.uart.address);
    uart.write_byte(b'\n');

    uart.write_str("interrupt-controller=");
    write_hex_u64(&mut uart, platform_info.interrupt_controller.address);
    uart.write_byte(b'\n');

    kernel::start(&boot_info);
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
