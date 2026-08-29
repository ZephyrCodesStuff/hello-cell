/* ps3.ld */
OUTPUT_FORMAT("elf64-powerpc", "elf64-powerpc", "elf64-powerpc")
OUTPUT_ARCH(powerpc:common64)
ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5);  /* RX */
    data PT_LOAD FLAGS(6);  /* RW */
}

SECTIONS
{
    /* Text Segment (64 KB Page Aligned) */
    . = 0x10000;

    .text : {
        _start_code = .;
        *(.text._start_code)
        *(.text .text.*)
        *(.rodata .rodata.*)
    } :text

    /* Data Segment (64 KB Boundary) */
    . = ALIGN(0x10000);

    .opd : ALIGN(8) {
        /* Construct the 24-byte ELFv1 entry descriptor for LV2 Kernel */
        _start = .;
        QUAD(_start_code)
        QUAD(.TOC.)
        QUAD(0)

        KEEP(*(.opd .opd.*))
    } :data

    .got : ALIGN(8) {
        *(.got .got.*)
        *(.toc .toc.*)
    } :data

    .data : ALIGN(8) {
        *(.data.rel.ro .data.rel.ro.*)
        *(.data .data.*)
    } :data

    .bss : ALIGN(8) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } :data

    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}

PROVIDE(.TOC. = ADDR(.got) + 0x8000);