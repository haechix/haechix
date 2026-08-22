OUTPUT_ARCH(aarch64)
ENTRY(_start)

KERNEL_LOAD_ADDRESS = 0x40080000;
BOOT_STACK_SIZE = 64K;

SECTIONS
{
    . = KERNEL_LOAD_ADDRESS;
    __kernel_start = .;

    .text.boot : ALIGN(4K)
    {
        KEEP(*(.text.boot))
    }

    .text.exception_vector : ALIGN(2K)
    {
        KEEP(*(.text.exception_vector))
    }

    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    .bss (NOLOAD) : ALIGN(16)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    .boot_stack (NOLOAD) : ALIGN(16)
    {
        __boot_stack_bottom = .;
        . += BOOT_STACK_SIZE;
        __boot_stack_top = .;
    }
    /DISCARD/ :
    {
        *(.eh_frame .eh_frame.*)
        *(.eh_frame_hdr)
    }

    __kernel_end = .;
}
