.section .text.boot, "ax"
.balign 4

.global _start
.type _start, %function

// Entry invariant: no instruction may use the stack before SP is initialized.

_start:
    // Mask debug, SError, IRQ, and FIQ before exception vectors exist.
    // D: Debug, A: SError, I: IRQ, F: FIQ
    msr daifset, #0xf

    // Rust's aarch64-unknown-none target may emit FP/Advanced SIMD instructions.
    // QEMU enters this M02 boot path at EL1, so enable EL1 access before Rust.
    mrs x9, cpacr_el1
    orr x9, x9, #0x300000
    msr cpacr_el1, x9
    isb

    // The linker reserves a 16-byte-aligned 64 KiB boot stack.
    adrp x9, __boot_stack_top
    add x9, x9, :lo12:__boot_stack_top
    mov sp, x9

    // The linker guarantees that this half-open range is writable RAM.
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
    // Enter Rust after the stack and .bss are initialized.
    bl start

.Lpark:
    wfe
    b .Lpark

.size _start, . - _start