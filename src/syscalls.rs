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
            "sc",
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
        "sc",
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
        "sc",
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
            "sc",
            "addi 1, 1, 128",
            "mr 2, 22",
            out("r22") _saved_r2,
            in("r11") 141u64,         // SYS_TIMER_USLEEP
            in("r3") usecs,
            clobber_abi("C"),
        );
    }
}

/// Creates a memory container for PRX / system allocations (Syscall 324: SYSCALL_MEMORY_CONTAINER_CREATE).
pub unsafe fn sys_memory_container_create(size: usize) -> Result<u32, i32> {
    let mut container: u32 = 0;
    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 324u64,         // SYSCALL_MEMORY_CONTAINER_CREATE
        in("r3") (&mut container as *mut u32) as usize as u64,
        in("r4") size as u64,
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret == 0 {
        Ok(container)
    } else {
        Err(ret)
    }
}

/// Loads a PRX module into the current process using a memory container (Syscall 497: SYSCALL_PRX_LOAD_MODULE_ON_MEMCONTAINER).
///
/// Verified directly from Sony `liblv2.prx` (sub_11928):
/// * r3 = path
/// * r4 = memory container ID
/// * r5 = flags (0)
/// * r6 = opt (0)
/// * r11 = 497 (0x1F1)
pub unsafe fn sys_prx_load_module_on_memcontainer(
    path: &str,
    container: u32,
    flags: u64,
) -> Result<i32, i32> {
    let mut path_buf = [0u8; 256];
    let len = path.len().min(255);
    path_buf[..len].copy_from_slice(&path.as_bytes()[..len]);
    path_buf[len] = 0;

    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 497u64,         // SYSCALL_PRX_LOAD_MODULE_ON_MEMCONTAINER
        in("r3") path_buf.as_ptr() as usize as u64,
        in("r4") container as u64,
        in("r5") flags,
        in("r6") 0u64,            // opt (NULL)
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret >= 0 {
        Ok(ret)
    } else {
        Err(ret)
    }
}

/// Loads a PRX module into the current process (Syscall 480: SYSCALL_PRX_LOAD_MODULE).
pub unsafe fn sys_prx_load_module(
    path: &str,
    flags: u64,
) -> Result<i32, i32> {
    let mut path_buf = [0u8; 256];
    let len = path.len().min(255);
    path_buf[..len].copy_from_slice(&path.as_bytes()[..len]);
    path_buf[len] = 0;

    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 480u64,         // SYSCALL_PRX_LOAD_MODULE
        in("r3") path_buf.as_ptr() as usize as u64,
        in("r4") flags,
        in("r5") 0u64,
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret >= 0 {
        Ok(ret)
    } else {
        Err(ret)
    }
}

/// Starts an already loaded PRX module (Syscall 481: SYSCALL_PRX_START_MODULE).
pub unsafe fn sys_prx_start_module(prx_id: i32) -> Result<i32, i32> {
    let mut modres: i32 = 0;
    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 481u64,         // SYSCALL_PRX_START_MODULE
        in("r3") prx_id as u64,
        in("r4") 0u64,            // args
        in("r5") 0u64,            // argp
        in("r6") (&mut modres as *mut i32) as usize as u64,
        in("r7") 0u64,            // flags
        in("r8") 0u64,            // opt
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret == 0 {
        Ok(modres)
    } else {
        Err(ret)
    }
}

/// Queries a module ID by name (Syscall 496: SYSCALL_PRX_GET_MODULE_ID_BY_NAME).
pub unsafe fn sys_prx_get_module_id_by_name(name: &str) -> Result<i32, i32> {
    let mut name_buf = [0u8; 64];
    let len = name.len().min(63);
    name_buf[..len].copy_from_slice(&name.as_bytes()[..len]);
    name_buf[len] = 0;

    let ret: i32;
    let mut _saved_r2: u64;

    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") 496u64,         // SYSCALL_PRX_GET_MODULE_ID_BY_NAME
        in("r3") name_buf.as_ptr() as usize as u64,
        in("r4") 0u64,
        in("r5") 0u64,
        lateout("r3") ret,
        clobber_abi("C"),
    );

    if ret >= 0 {
        Ok(ret)
    } else {
        Err(ret)
    }
}

/// Terminates the current process (Syscall 3: SYS_PROCESS_EXIT).
pub fn sys_process_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "mr 3, {0}",
            "li 11, 3",
            "sc",
            in(reg) status as u64,
            options(noreturn)
        );
    }
}
