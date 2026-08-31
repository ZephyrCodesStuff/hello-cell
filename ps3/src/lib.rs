//! PlayStation 3 Rust Runtime & SDK
//!
//! Provides core language runtime, heap allocation, TTY output, LV2 syscalls,
//! and network socket support for the Sony PlayStation 3 (CellOS PPU).

#![no_std]

pub extern crate alloc;

pub mod allocator;
pub mod io;
pub mod mem;
pub mod net;
pub mod panic;
pub mod sys;

pub use allocator::{get_heap_stats, HeapStats};
