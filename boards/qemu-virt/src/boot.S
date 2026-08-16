.section .text.boot, "ax"
.balign 4

.global _start
.type _start, %function

// Entry invariant: no instruction may use the stack before EL normalization
// installs the board-provided boot stack.

_start:
    // Mask debug, SError, IRQ, and FIQ before exception vectors exist.
    // D: Debug, A: SError, I: IRQ, F: FIQ
    msr daifset, #0xf

    // Supply the board-owned 16-byte-aligned boot stack to the reusable
    // architecture normalization routine. x0, containing the DTB address,
    // must remain unchanged.
    adrp x1, __boot_stack_top
    add x1, x1, :lo12:__boot_stack_top
    bl aarch64_normalize_to_el1

    // x1 reports the original Exception Level. Both supported paths return
    // here at EL1h with the boot stack installed.
    cmp x1, #1
    b.eq .Lnormalized_el1

    cmp x1, #2
    b.eq .Lnormalized_el1

    cmp x1, #3
    b.eq .Lunsupported_el3

    // EL0 cannot read CurrentEL and is outside the supported boot contract.
    b .Lpark

.Lnormalized_el1:
    // Rust's aarch64-unknown-none target may emit FP/Advanced SIMD
    // instructions. Enable EL1 access only after EL normalization.
    mrs x9, cpacr_el1
    orr x9, x9, #0x300000
    msr cpacr_el1, x9
    isb

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
    // Enter Rust only after EL1h, the stack, FP/SIMD, and .bss are ready.
    bl start
    b .Lpark

.Lunsupported_el3:
    // This is a board-specific early diagnostic path. It cannot use Rust,
    // the stack, or the reusable driver because EL1 normalization failed.
    adrp x9, .Lunsupported_el3_message
    add x9, x9, :lo12:.Lunsupported_el3_message

    // QEMU virt PL011 base address: 0x0900_0000.
    movz x10, #0x0900, lsl #16

.Lunsupported_el3_next_byte:
    ldrb w11, [x9], #1
    cbz w11, .Lpark

.Lunsupported_el3_wait:
    // PL011 FR offset 0x18, TXFF bit 5.
    ldr w12, [x10, #0x18]
    tbnz w12, #5, .Lunsupported_el3_wait

    str w11, [x10]
    b .Lunsupported_el3_next_byte

.Lpark:
    wfe
    b .Lpark

.size _start, . - _start

.section .rodata.boot, "a"
.Lunsupported_el3_message:
    .asciz "Haechix M03: unsupported EL3\n"
