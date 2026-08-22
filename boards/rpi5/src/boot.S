.section .text.boot, "ax"
.balign 4

.global _start
.type _start, %function

// Raspberry Pi firmware boot contract:
//
//   x0 = physical DTB address
//   MMU = disabled
//   entry = non-secure EL1 or EL2
//
// No instruction may use the stack before EL normalization installs
// __boot_stack_top as SP_EL1.

_start:
    // Mask Debug, SError, IRQ, and FIQ before exception vectors exist.
    msr daifset, #0xf

    // Run only affinity-level-0 core zero during single-core bring-up.
    mrs x9, mpidr_el1
    and x9, x9, #0xff
    cbnz x9, .Lpark

    // Marker 1: primary core reached the Pi 5 entry point.
    mov w9, #0x31
    bl .Learly_uart_write

    // Preserve x0, which contains the firmware-provided DTB address.
    adrp x1, __boot_stack_top
    add x1, x1, :lo12:__boot_stack_top
    bl aarch64_normalize_to_el1

    // x1 contains the original Exception Level.
    cmp x1, #1
    b.eq .Lnormalized_el1

    cmp x1, #2
    b.eq .Lnormalized_el1

    cmp x1, #3
    b.eq .Lunsupported_el3

    b .Lpark

.Lnormalized_el1:
    // Marker 2: EL1h and the EL1 boot stack are active.
    mov w9, #0x32
    bl .Learly_uart_write

    // Rust may emit FP/Advanced SIMD instructions.
    mrs x9, cpacr_el1
    orr x9, x9, #0x300000
    msr cpacr_el1, x9
    isb

    // Marker 3: EL1 FP/Advanced SIMD access is enabled.
    mov w9, #0x33
    bl .Learly_uart_write

    adrp x9, __bss_start
    add x9, x9, :lo12:__bss_start

    adrp x10, __bss_end
    add x10, x10, :lo12:__bss_end

.Lclear_bss:
    cmp x9, x10
    b.hs .Lbss_cleared

    strb wzr, [x9], #1
    b .Lclear_bss

.Lbss_cleared:
    // Marker 4: .bss has been zeroed.
    mov w9, #0x34
    bl .Learly_uart_write

    // Marker 5: Rust board start() is about to be called.
    mov w9, #0x35
    bl .Learly_uart_write

    mov w9, #0x0a
    bl .Learly_uart_write

    bl start

.Lpark:
    wfe
    b .Lpark

.Lunsupported_el3:
    mov w9, #0x45
    bl .Learly_uart_write

    mov w9, #0x33
    bl .Learly_uart_write

    mov w9, #0x0a
    bl .Learly_uart_write

    b .Lpark

// Stackless firmware-initialized UART10 output.
//
// Input:
//   w9 = byte
//
// Clobbers:
//   x10, w11
//
// Preserves:
//   x0 = DTB address
.Learly_uart_write:
    // Raspberry Pi 5 UART10 PL011 CPU physical address:
    // 0x00000010_7d001000
    movz x10, #0x1000
    movk x10, #0x7d00, lsl #16
    movk x10, #0x0010, lsl #32

.Learly_uart_wait:
    // PL011 FR offset 0x18, TXFF bit 5.
    ldr w11, [x10, #0x18]
    tbnz w11, #5, .Learly_uart_wait

    str w9, [x10]
    ret

.size _start, . - _start
