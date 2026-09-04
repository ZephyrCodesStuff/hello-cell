//! PS3 LV2 Process Bootstrapping & Termination Support

use core::arch::global_asm;

/// Trait implemented by types that can be returned by `#[ps3::main]`.
pub trait Termination {
    fn report(self) -> i32;
}

impl Termination for () {
    #[inline(always)]
    fn report(self) -> i32 {
        0
    }
}

impl Termination for i32 {
    #[inline(always)]
    fn report(self) -> i32 {
        self
    }
}

impl Termination for u32 {
    #[inline(always)]
    fn report(self) -> i32 {
        self as i32
    }
}

impl Termination for ! {
    #[inline(always)]
    fn report(self) -> i32 {
        self
    }
}

impl<E: core::fmt::Debug> Termination for Result<(), E> {
    fn report(self) -> i32 {
        match self {
            Ok(()) => 0,
            Err(e) => {
                crate::println!("Process terminated with error: {:?}", e);
                1
            }
        }
    }
}

global_asm!(include_str!(concat!(env!("OUT_DIR"), "/sprx.s")));
