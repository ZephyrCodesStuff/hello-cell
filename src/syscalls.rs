//! Safe wrappers for PlayStation 3 Level 2 (LV2) Kernel Syscalls.
//!
//! Every syscall preserves the TOC register (`r2`) and creates a standard
//! 128-byte parameter/linkage stack frame.

use core::arch::asm;

// -----------------------------------------------------------------------------
// Memory Flags & Page Constants
// -----------------------------------------------------------------------------

pub const SYS_MEMORY_PAGE_SIZE_64K: u64 = 0x200;
pub const SYS_MEMORY_PAGE_SIZE_1M: u64 = 0x400;

// -----------------------------------------------------------------------------
// Low-Level Syscall Primitive & Variadic Macro
// -----------------------------------------------------------------------------

/// The single low-level PS3 LV2 syscall primitive.
///
/// Sets up a 128-byte parameter/linkage stack frame, preserves the TOC register
/// (`r2`) into `r22`, loads the syscall number into `r11`, places arguments in
/// `r3`..`r10`, and executes `sc`.
#[inline(always)]
pub unsafe fn raw_syscall(
    nr: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    a8: u64,
) -> i64 {
    let mut _saved_r2: u64;
    let ret: i64;
    asm!(
        "mr 22, 2",
        "stdu 1, -128(1)",
        "sc",
        "addi 1, 1, 128",
        "mr 2, 22",
        out("r22") _saved_r2,
        in("r11") nr,
        in("r3") a1,
        in("r4") a2,
        in("r5") a3,
        in("r6") a4,
        in("r7") a5,
        in("r8") a6,
        in("r9") a7,
        in("r10") a8,
        lateout("r3") ret,
        clobber_abi("C"),
    );
    ret
}

/// Variadic LV2 syscall macro supporting 0 to 8 arguments.
#[macro_export]
macro_rules! lv2_syscall {
    ($nr:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, 0, 0, 0, 0, 0, 0, 0, 0)
    };
    ($nr:expr, $a1:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, 0, 0, 0, 0, 0, 0, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, 0, 0, 0, 0, 0, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, $a3 as usize as u64, 0, 0, 0, 0, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, $a3 as usize as u64, $a4 as usize as u64, 0, 0, 0, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, $a3 as usize as u64, $a4 as usize as u64, $a5 as usize as u64, 0, 0, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, $a3 as usize as u64, $a4 as usize as u64, $a5 as usize as u64, $a6 as usize as u64, 0, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, $a3 as usize as u64, $a4 as usize as u64, $a5 as usize as u64, $a6 as usize as u64, $a7 as usize as u64, 0)
    };
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr) => {
        $crate::syscalls::raw_syscall($nr as u64, $a1 as usize as u64, $a2 as usize as u64, $a3 as usize as u64, $a4 as usize as u64, $a5 as usize as u64, $a6 as usize as u64, $a7 as usize as u64, $a8 as usize as u64)
    };
}

// -----------------------------------------------------------------------------
// Declarative Syscall Definition DSL
// -----------------------------------------------------------------------------

/// Defines syscall number constants and low-level typed functions
/// from a single declarative source of truth.
macro_rules! define_syscalls {
    (
        $(
            $(#[$meta:meta])*
            pub unsafe fn $name:ident ( $( $arg:ident : $arg_ty:ty ),* $(,)? ) $(-> $ret:ty)? = $const_name:ident : $nr:literal;
        )*
    ) => {
        pub mod nr {
            //! PS3 LV2 Kernel Syscall Numbers
            $(
                $(#[$meta])*
                pub const $const_name: u64 = $nr;
            )*
        }

        $(
            $(#[$meta])*
            #[inline(always)]
            pub unsafe fn $name ( $( $arg : $arg_ty ),* ) $(-> $ret)? {
                $crate::lv2_syscall!(
                    $crate::syscalls::nr::$const_name
                    $(, $arg )*
                ) $(as $ret)?
            }
        )*
    };
}

// -----------------------------------------------------------------------------
// Syscall Table (Single Source of Truth)
// -----------------------------------------------------------------------------

define_syscalls! {
    /// Terminates the current process (Syscall 3: SYS_PROCESS_EXIT).
    pub unsafe fn sys_process_exit_raw(status: i32) -> i32 = SYS_PROCESS_EXIT : 3;

    /// Exits the current PPU thread (Syscall 41: SYS_PPU_THREAD_EXIT).
    pub unsafe fn sys_ppu_thread_exit(val: u64) -> i32 = SYS_PPU_THREAD_EXIT : 41;

    /// Creates a PPU thread (Syscall 52: SYS_PPU_THREAD_CREATE).
    pub unsafe fn sys_ppu_thread_create(thread_id: *mut u64, entry: u64, arg: u64, prio: i32, stacksize: usize, flags: u64, threadname: *const u8) -> i32 = SYS_PPU_THREAD_CREATE : 52;

    /// Sleeps the current PPU thread for microseconds (Syscall 141: SYS_TIMER_USLEEP).
    pub unsafe fn sys_timer_usleep_raw(usecs: u64) -> i32 = SYS_TIMER_USLEEP : 141;

    /// Creates a memory container for PRX allocations (Syscall 324: SYSCALL_MEMORY_CONTAINER_CREATE).
    pub unsafe fn sys_memory_container_create_raw(out_container: *mut u32, size: usize) -> i32 = SYS_MEMORY_CONTAINER_CREATE : 324;

    /// Destroys a memory container (Syscall 325: SYSCALL_MEMORY_CONTAINER_DESTROY).
    pub unsafe fn sys_memory_container_destroy(container: u32) -> i32 = SYS_MEMORY_CONTAINER_DESTROY : 325;

    /// Allocates virtual memory pages from LV2 (Syscall 348: SYS_MEMORY_ALLOCATE).
    pub unsafe fn sys_memory_allocate_raw(size: usize, flags: u64, out_addr: *mut u32) -> i32 = SYS_MEMORY_ALLOCATE : 348;

    /// Frees virtual memory allocated with sys_memory_allocate (Syscall 349: SYS_MEMORY_FREE).
    pub unsafe fn sys_memory_free(start_addr: *mut u8) -> i32 = SYS_MEMORY_FREE : 349;

    /// Queries total and available user memory (Syscall 352: SYS_MEMORY_GET_USER_MEMORY_SIZE).
    pub unsafe fn sys_memory_get_user_memory_size_raw(info: *mut [u32; 2]) -> i32 = SYS_MEMORY_GET_USER_MEMORY_SIZE : 352;

    /// Reads from a TTY channel (Syscall 402: SYS_TTY_READ).
    pub unsafe fn sys_tty_read(channel: u64, buf: *mut u8, len: usize, read_len: *mut u32) -> i32 = SYS_TTY_READ : 402;

    /// Writes data to a TTY channel (Syscall 403: SYS_TTY_WRITE).
    pub unsafe fn sys_tty_write_raw(channel: u64, ptr: *const u8, len: usize) -> i32 = SYS_TTY_WRITE : 403;

    /// Loads a PRX module into the process (Syscall 480: SYSCALL_PRX_LOAD_MODULE).
    pub unsafe fn sys_prx_load_module_raw(path: *const u8, flags: u64, opt: u64) -> i32 = SYS_PRX_LOAD_MODULE : 480;

    /// Starts an already loaded PRX module (Syscall 481: SYSCALL_PRX_START_MODULE).
    pub unsafe fn sys_prx_start_module_raw(prx_id: i32, args: u64, argp: u64, modres: *mut i32, flags: u64, opt: u64) -> i32 = SYS_PRX_START_MODULE : 481;

    /// Stops a running PRX module (Syscall 482: SYSCALL_PRX_STOP_MODULE).
    pub unsafe fn sys_prx_stop_module(prx_id: i32, args: u64, argp: u64, modres: *mut i32, flags: u64, opt: u64) -> i32 = SYS_PRX_STOP_MODULE : 482;

    /// Unloads a PRX module (Syscall 483: SYSCALL_PRX_UNLOAD_MODULE).
    pub unsafe fn sys_prx_unload_module(prx_id: i32, flags: u64, opt: u64) -> i32 = SYS_PRX_UNLOAD_MODULE : 483;

    /// Queries a module ID by name (Syscall 496: SYSCALL_PRX_GET_MODULE_ID_BY_NAME).
    pub unsafe fn sys_prx_get_module_id_by_name_raw(name: *const u8, flags: u64, opt: u64) -> i32 = SYS_PRX_GET_MODULE_ID_BY_NAME : 496;

    /// Loads a PRX module onto a memory container (Syscall 497: SYSCALL_PRX_LOAD_MODULE_ON_MEMCONTAINER).
    pub unsafe fn sys_prx_load_module_on_memcontainer_raw(path: *const u8, container: u32, flags: u64, opt: u64) -> i32 = SYS_PRX_LOAD_MODULE_ON_MEMCONTAINER : 497;

    /// Opens a file in the GameOS VFS (Syscall 801: SYS_FS_OPEN).
    pub unsafe fn sys_fs_open(path: *const u8, flags: i32, fd: *mut i32, mode: u32, arg: *const u8, size: u64) -> i32 = SYS_FS_OPEN : 801;

    /// Reads from a file descriptor (Syscall 802: SYS_FS_READ).
    pub unsafe fn sys_fs_read(fd: i32, buf: *mut u8, nbytes: u64, nread: *mut u64) -> i32 = SYS_FS_READ : 802;

    /// Writes to a file descriptor (Syscall 803: SYS_FS_WRITE).
    pub unsafe fn sys_fs_write(fd: i32, buf: *const u8, nbytes: u64, nwritten: *mut u64) -> i32 = SYS_FS_WRITE : 803;

    /// Closes a file descriptor (Syscall 804: SYS_FS_CLOSE).
    pub unsafe fn sys_fs_close(fd: i32) -> i32 = SYS_FS_CLOSE : 804;
}

// -----------------------------------------------------------------------------
// Ergonomic Rust Wrappers (Handling Strings, Buffers & Result Types)
// -----------------------------------------------------------------------------

/// Writes a string slice to the TTY debug console (stdout channel 0).
pub fn sys_tty_write(msg: &str) {
    unsafe {
        sys_tty_write_raw(0, msg.as_ptr(), msg.len());
    }
}

/// Sleeps the current PPU thread for the given number of microseconds.
pub fn sys_timer_usleep(usecs: u64) {
    unsafe {
        sys_timer_usleep_raw(usecs);
    }
}

/// Allocates virtual memory pages from the LV2 kernel.
pub unsafe fn sys_memory_allocate(size: usize, flags: u64) -> Result<*mut u8, i32> {
    let mut out_addr: u32 = 0;
    let ret = sys_memory_allocate_raw(size, flags, &mut out_addr);

    if ret == 0 {
        Ok(out_addr as usize as *mut u8)
    } else {
        Err(ret)
    }
}

/// Queries total and available user memory.
///
/// Returns `(total_bytes, available_bytes)` on success.
pub unsafe fn sys_memory_get_user_memory_size() -> Result<(u32, u32), i32> {
    let mut mem_info: [u32; 2] = [0, 0];
    let ret = sys_memory_get_user_memory_size_raw(&mut mem_info);

    if ret == 0 {
        Ok((mem_info[0], mem_info[1]))
    } else {
        Err(ret)
    }
}

/// Creates a memory container for PRX / system allocations.
pub unsafe fn sys_memory_container_create(size: usize) -> Result<u32, i32> {
    let mut container: u32 = 0;
    let ret = sys_memory_container_create_raw(&mut container, size);

    if ret == 0 {
        Ok(container)
    } else {
        Err(ret)
    }
}

/// Loads a PRX module into the process using a memory container.
pub unsafe fn sys_prx_load_module_on_memcontainer(
    path: &str,
    container: u32,
    flags: u64,
) -> Result<i32, i32> {
    let mut path_buf = [0u8; 256];
    let len = path.len().min(255);
    path_buf[..len].copy_from_slice(&path.as_bytes()[..len]);
    path_buf[len] = 0;

    let ret = sys_prx_load_module_on_memcontainer_raw(path_buf.as_ptr(), container, flags, 0);

    if ret >= 0 {
        Ok(ret)
    } else {
        Err(ret)
    }
}

/// Loads a PRX module into the process from a path string.
pub unsafe fn sys_prx_load_module(
    path: &str,
    flags: u64,
) -> Result<i32, i32> {
    let mut path_buf = [0u8; 256];
    let len = path.len().min(255);
    path_buf[..len].copy_from_slice(&path.as_bytes()[..len]);
    path_buf[len] = 0;

    let ret = sys_prx_load_module_raw(path_buf.as_ptr(), flags, 0);

    if ret >= 0 {
        Ok(ret)
    } else {
        Err(ret)
    }
}

/// Starts an already loaded PRX module.
pub unsafe fn sys_prx_start_module(prx_id: i32) -> Result<i32, i32> {
    let mut modres: i32 = 0;
    let ret = sys_prx_start_module_raw(prx_id, 0, 0, &mut modres, 0, 0);

    if ret == 0 {
        Ok(modres)
    } else {
        Err(ret)
    }
}

/// Queries a module ID by name string.
pub unsafe fn sys_prx_get_module_id_by_name(name: &str) -> Result<i32, i32> {
    let mut name_buf = [0u8; 64];
    let len = name.len().min(63);
    name_buf[..len].copy_from_slice(&name.as_bytes()[..len]);
    name_buf[len] = 0;

    let ret = sys_prx_get_module_id_by_name_raw(name_buf.as_ptr(), 0, 0);

    if ret >= 0 {
        Ok(ret)
    } else {
        Err(ret)
    }
}

/// Terminates the current process (Syscall 3: SYS_PROCESS_EXIT).
pub fn sys_process_exit(status: i32) -> ! {
    unsafe {
        sys_process_exit_raw(status);
        loop {}
    }
}
