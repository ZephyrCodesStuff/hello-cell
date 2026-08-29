//! Panic handling for PS3 barebones Rust.

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::println!("\n====================");
    crate::println!("[PANIC] {}", info);
    crate::println!("====================");
    loop {}
}
