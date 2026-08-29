//! Safe wrappers for PlayStation 3 Level 2 (LV2) Kernel Syscalls.
//!
//! Every syscall preserves the TOC register (`r2`) and creates a standard
//! 128-byte parameter/linkage stack frame.

use core::arch::asm;

pub const SYS_MEMORY_PAGE_SIZE_64K: u64 = 0x200;
pub const SYS_MEMORY_PAGE_SIZE_1M: u64 = 0x400;

/// Writes a string to the TTY / ProDG debug console (Syscall 403: SYS_TTY_WRITE).
pub fn sys_tty_write(msg: &str) {
    unsafe {
        let mut _saved_r2: u64;
        asm!(
            "mr 22, 2",
            "stdu 1, -128(1)",
            "sc 2",
            "addi 1, 1, 128",
            "mr 2, 22",
            out("r22") _saved_r2,
            in("r11") 403u64,         // SYS_TTY_WRITE
            in("r3") 0u64,            // Channel 0 (stdout)
            in("r4") msg.as_ptr() as usize as u64,
            in("r5") msg.len() as u64,
            clobber_abi("C"),
        );
    }
}

/// Allocates virtual memory pages from the LV2 kernel (Syscall 348: SYS_MEMORY_ALLOCATE).
///
/// # Arguments
/// * `size` - Size in bytes (must be aligned to page size: 64KB or 1MB).
/// * `flags` - Page size flags (e.g. `SYS_MEMORY_PAGE_SIZE_64K`).
pub unsafe fn sys_memory_allocate(size: usize, flags: u64) -> Result<*mut u8, i32> {
    let mut out_addr: u32 = 0;
    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc 2",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 348u64,         // SYS_MEMORY_ALLOCATE
        in("r3") size as u64,
        in("r4") flags,
        in("r5") (&mut out_addr as *mut u32) as usize as u64,
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret == 0 {
        Ok(out_addr as usize as *mut u8)
    } else {
        Err(ret)
    }
}

/// Frees virtual memory allocated with `sys_memory_allocate` (Syscall 349: SYS_MEMORY_FREE).
pub unsafe fn sys_memory_free(start_addr: *mut u8) -> Result<(), i32> {
    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc 2",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 349u64,         // SYS_MEMORY_FREE
        in("r3") start_addr as usize as u64,
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret == 0 {
        Ok(())
    } else {
        Err(ret)
    }
}

/// Sleeps the current PPU thread for the given number of microseconds (Syscall 141: SYS_TIMER_USLEEP).
pub fn sys_timer_usleep(usecs: u64) {
    unsafe {
        let mut _saved_r2: u64;
        asm!(
            "mr 22, 2",
            "stdu 1, -128(1)",
            "sc 2",
            "addi 1, 1, 128",
            "mr 2, 22",
            out("r22") _saved_r2,
            in("r11") 141u64,         // SYS_TIMER_USLEEP
            in("r3") usecs,
            clobber_abi("C"),
        );
    }
}

/// Terminates the current process (Syscall 3: SYS_PROCESS_EXIT).
pub fn sys_process_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "mr 3, {0}",
            "li 11, 3",
            "sc 2",
            in(reg) status as u64,
            options(noreturn)
        );
    }
}
