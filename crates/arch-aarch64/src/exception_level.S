.section .text.arch.exception_level, "ax"
.balign 4

.global aarch64_normalize_to_el1
.type aarch64_normalize_to_el1, %function

// Normalize the boot Exception Level without using the stack.
//
// Input:
//   x0 = boot DTB address; preserved for the later boot protocol
//   x1 = 16-byte-aligned EL1 boot stack top
//   x30 = return address supplied by the board entry with BL
//
// Output:
//   x0 = preserved boot DTB address
//   x1 = original Exception Level encoding: 1, 2, or 3
//
// Clobbers:
//   x9, x10
//
// Preconditions:
//   - execution is at EL1 or higher
//   - interrupts are masked
//   - x1 points to valid writable boot-stack memory
aarch64_normalize_to_el1:
    mrs x9, CurrentEL
    ubfx x9, x9, #2, #2

    cmp x9, #1
    b.eq .Lfrom_el1

    cmp x9, #2
    b.eq .Lfrom_el2

    // EL3 is unsupported and must be handled by the board.
    // EL0 cannot validly execute the CurrentEL read above.
    mov x1, x9
    ret

.Lfrom_el1:
    // EL1h uses SP_EL1, represented by SP while executing at EL1.
    mov sp, x1
    mov x1, #1
    ret

.Lfrom_el2:
    // Install the stack that becomes active after entering EL1h.
    msr sp_el1, x1

    // Execute EL1 in AArch64 state while preserving unrelated HCR bits.
    mrs x10, hcr_el2
    orr x10, x10, #0x80000000
    msr hcr_el2, x10

    // Do not trap EL1 FP or Advanced SIMD instructions to EL2.
    mrs x10, cptr_el2
    bic x10, x10, #0x400
    msr cptr_el2, x10

    // Return to AArch64 EL1h with D, A, I, and F masked.
    mov x10, #0x3c5
    msr spsr_el2, x10
    msr elr_el2, x30

    mov x1, #2
    isb
    eret

.size aarch64_normalize_to_el1, . - aarch64_normalize_to_el1
