.section .text.exception_vector, "ax", %progbits
.balign 2048

.global aarch64_exception_vector_table
.type aarch64_exception_vector_table, %function

// Preserve x16 and x17 before using x16 as the vector identifier.
//
// Each architectural vector slot is exactly 128 bytes. The common entry
// expands the temporary 16-byte save into the complete exception frame.
.macro VECTOR_SLOT vector_id
    sub     sp, sp, #16
    stp     x16, x17, [sp]
    mov     x16, #\vector_id
    b       aarch64_exception_entry
    .balign 128
.endm

aarch64_exception_vector_table:
    // Current EL using SP0.
    VECTOR_SLOT 0
    VECTOR_SLOT 1
    VECTOR_SLOT 2
    VECTOR_SLOT 3

    // Current EL using SPx.
    VECTOR_SLOT 4
    VECTOR_SLOT 5
    VECTOR_SLOT 6
    VECTOR_SLOT 7

    // Lower EL using AArch64.
    VECTOR_SLOT 8
    VECTOR_SLOT 9
    VECTOR_SLOT 10
    VECTOR_SLOT 11

    // Lower EL using AArch32.
    VECTOR_SLOT 12
    VECTOR_SLOT 13
    VECTOR_SLOT 14
    VECTOR_SLOT 15

.global aarch64_exception_vector_table_end
aarch64_exception_vector_table_end:

.size aarch64_exception_vector_table, aarch64_exception_vector_table_end - aarch64_exception_vector_table

.section .text.exception_entry, "ax", %progbits
.balign 4

// ExceptionContext layout.
.equ CONTEXT_X0,               0
.equ CONTEXT_X16,            128
.equ CONTEXT_X17,            136
.equ CONTEXT_X18,            144
.equ CONTEXT_X30,            240
.equ CONTEXT_SP_AT_ENTRY,    248
.equ CONTEXT_ELR_EL1,        256
.equ CONTEXT_SPSR_EL1,       264
.equ CONTEXT_ESR_EL1,        272
.equ CONTEXT_FAR_EL1,        280
.equ CONTEXT_VECTOR_ID,      288
.equ CONTEXT_RESERVED,       296
.equ EXCEPTION_CONTEXT_SIZE, 304

// Assembly-private FP and Advanced SIMD state.
.equ CONTEXT_Q0,             304
.equ CONTEXT_FPCR,           816
.equ CONTEXT_FPSR,           824
.equ EXCEPTION_FRAME_SIZE,   832

// The vector slot already reserved sixteen bytes for x16 and x17.
.equ FRAME_REMAINING_SIZE,   816

.extern haechix_exception_dispatch

.global aarch64_exception_entry
.type aarch64_exception_entry, %function

aarch64_exception_entry:
    // Expand the temporary x16/x17 save into the complete 832-byte frame.
    sub     sp, sp, #FRAME_REMAINING_SIZE

    // The original x16 and x17 are temporarily located at the last sixteen
    // bytes of the complete frame. Copy them into ExceptionContext before
    // those locations are reused for FPCR and FPSR.
    ldr     x17, [sp, #CONTEXT_FPCR]
    str     x17, [sp, #CONTEXT_X16]
    ldr     x17, [sp, #CONTEXT_FPSR]
    str     x17, [sp, #CONTEXT_X17]

    // x16 currently contains the vector identifier supplied by the slot.
    str     x16, [sp, #CONTEXT_VECTOR_ID]
    str     xzr, [sp, #CONTEXT_RESERVED]

    // Save x0 through x15.
    stp     x0, x1, [sp, #CONTEXT_X0]
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]
    stp     x8, x9, [sp, #64]
    stp     x10, x11, [sp, #80]
    stp     x12, x13, [sp, #96]
    stp     x14, x15, [sp, #112]

    // Save x18 through x29 and the exception-time link register.
    stp     x18, x19, [sp, #CONTEXT_X18]
    stp     x20, x21, [sp, #160]
    stp     x22, x23, [sp, #176]
    stp     x24, x25, [sp, #192]
    stp     x26, x27, [sp, #208]
    stp     x28, x29, [sp, #224]
    str     x30, [sp, #CONTEXT_X30]

    // Record the stack pointer as it existed before the vector slot reserved
    // any exception-frame storage.
    add     x17, sp, #EXCEPTION_FRAME_SIZE
    str     x17, [sp, #CONTEXT_SP_AT_ENTRY]

    // Capture the EL1 exception-return and syndrome registers.
    mrs     x17, ELR_EL1
    str     x17, [sp, #CONTEXT_ELR_EL1]
    mrs     x17, SPSR_EL1
    str     x17, [sp, #CONTEXT_SPSR_EL1]
    mrs     x17, ESR_EL1
    str     x17, [sp, #CONTEXT_ESR_EL1]
    mrs     x17, FAR_EL1
    str     x17, [sp, #CONTEXT_FAR_EL1]

    // Preserve all FP and Advanced SIMD registers before entering Rust.
    stp     q0, q1, [sp, #CONTEXT_Q0]
    stp     q2, q3, [sp, #336]
    stp     q4, q5, [sp, #368]
    stp     q6, q7, [sp, #400]
    stp     q8, q9, [sp, #432]
    stp     q10, q11, [sp, #464]
    stp     q12, q13, [sp, #496]
    stp     q14, q15, [sp, #528]
    stp     q16, q17, [sp, #560]
    stp     q18, q19, [sp, #592]
    stp     q20, q21, [sp, #624]
    stp     q22, q23, [sp, #656]
    stp     q24, q25, [sp, #688]
    stp     q26, q27, [sp, #720]
    stp     q28, q29, [sp, #752]
    stp     q30, q31, [sp, #784]

    mrs     x17, FPCR
    str     x17, [sp, #CONTEXT_FPCR]
    mrs     x17, FPSR
    str     x17, [sp, #CONTEXT_FPSR]

    // Pass a unique mutable pointer to ExceptionContext in x0.
    mov     x0, sp
    bl      haechix_exception_dispatch

    // Restore FPCR, FPSR, and all FP and Advanced SIMD registers.
    ldr     x16, [sp, #CONTEXT_FPCR]
    msr     FPCR, x16
    ldr     x16, [sp, #CONTEXT_FPSR]
    msr     FPSR, x16

    ldp     q0, q1, [sp, #CONTEXT_Q0]
    ldp     q2, q3, [sp, #336]
    ldp     q4, q5, [sp, #368]
    ldp     q6, q7, [sp, #400]
    ldp     q8, q9, [sp, #432]
    ldp     q10, q11, [sp, #464]
    ldp     q12, q13, [sp, #496]
    ldp     q14, q15, [sp, #528]
    ldp     q16, q17, [sp, #560]
    ldp     q18, q19, [sp, #592]
    ldp     q20, q21, [sp, #624]
    ldp     q22, q23, [sp, #656]
    ldp     q24, q25, [sp, #688]
    ldp     q26, q27, [sp, #720]
    ldp     q28, q29, [sp, #752]
    ldp     q30, q31, [sp, #784]

    // Use the possibly updated exception-return state written by Rust.
    ldr     x16, [sp, #CONTEXT_ELR_EL1]
    msr     ELR_EL1, x16
    ldr     x16, [sp, #CONTEXT_SPSR_EL1]
    msr     SPSR_EL1, x16

    // Restore the interrupted general-purpose register state.
    ldp     x0, x1, [sp, #CONTEXT_X0]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]
    ldp     x8, x9, [sp, #64]
    ldp     x10, x11, [sp, #80]
    ldp     x12, x13, [sp, #96]
    ldp     x14, x15, [sp, #112]

    ldp     x18, x19, [sp, #CONTEXT_X18]
    ldp     x20, x21, [sp, #160]
    ldp     x22, x23, [sp, #176]
    ldp     x24, x25, [sp, #192]
    ldp     x26, x27, [sp, #208]
    ldp     x28, x29, [sp, #224]
    ldr     x30, [sp, #CONTEXT_X30]

    // Restore x16 and x17 last because they are used as assembly scratch
    // registers throughout the return path.
    ldp     x16, x17, [sp, #CONTEXT_X16]

    add     sp, sp, #EXCEPTION_FRAME_SIZE
    eret

.size aarch64_exception_entry, . - aarch64_exception_entry