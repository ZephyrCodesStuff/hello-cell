//! Formatting & I/O primitives for PlayStation 3 TTY output.
//!
//! Provides standard `print!` and `println!` macros.

use crate::syscalls::sys_tty_write;

#[doc(hidden)]
pub fn _print_str(s: &str) {
    sys_tty_write(s);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::_print_str(alloc::format!($($arg)*).as_str())
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::io::_print_str("\n")
    };
    ($($arg:tt)*) => {
        $crate::io::_print_str(alloc::format!($($arg)*).as_str());
        $crate::io::_print_str("\n");
    };
}
