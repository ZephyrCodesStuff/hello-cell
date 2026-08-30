//! PS3 LV2 Kernel Entry Point
//!
//! Handles low-level process bootstrapping and terminates the process
//! via SYS_PROCESS_EXIT upon return from `rust_main`.

use core::arch::global_asm;

global_asm!(
    r#"
    .section ".text._start_code", "ax"
    .globl _start_code
    .type _start_code, @function
_start_code:
    # 1. Kernel loads r2 = .TOC. from _start descriptor
    # 2. Call rust_main using standard ELFv1 ABI
    bl      rust_main
    nop

    # 3. Terminate process via LV2 Syscall 3 (SYS_PROCESS_EXIT)
    mr      3, 3
    li      11, 3
    sc      2

.halt:
    b       .halt
    "#
);

global_asm!(include_str!("sprx.s"));

