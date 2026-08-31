//! Minimal Hello World and Memory Allocation Example for PS3

#![no_std]
#![no_main]

use ps3::alloc::boxed::Box;
use ps3::alloc::format;
use ps3::alloc::vec::Vec;
use ps3::allocator;
use ps3::println;
use ps3::sys::syscalls;

#[no_mangle]
pub extern "C" fn rust_main() -> i32 {
    println!("========================================");
    println!(" Hello PlayStation 3 from Rust!");
    println!("========================================");

    // Test Box allocation
    let boxed_val = Box::new(0x1337_4242_u64);
    println!("Boxed Value: {:#X}", *boxed_val);

    // Test Dynamic Vector
    let mut vec: Vec<u32> = Vec::new();
    for i in 0..10 {
        vec.push(i * 10);
    }
    println!("Vector Items: {:?}", vec.as_slice());
    println!("Vector Length: {}", vec.len());

    // Test Dynamic String formatting
    let formatted_str = format!("Formatted string dynamically: 0x{:X}", 0xCAFEBABE_u32);
    println!("{}", formatted_str.as_str());

    // Heap stats
    let stats = allocator::get_heap_stats();
    println!("--- Heap Statistics ---");
    println!("  Active Allocs: {}", stats.active_allocations);
    println!("  Active Bytes:  {} bytes", stats.active_bytes);
    println!("  Claimed Total: {} MB", stats.claimed_bytes / (1024 * 1024));

    // Memory Probe
    match unsafe { syscalls::sys_memory_get_user_memory_size() } {
        Ok((total, avail)) => {
            println!(
                " [MEM] Total User Memory: {} MB, Available: {} MB",
                total / (1024 * 1024),
                avail / (1024 * 1024)
            );
        }
        Err(ret) => {
            println!(
                " [MEM] SYS_MEMORY_GET_USER_MEMORY_SIZE error: {:#X}",
                ret as u32
            );
        }
    }

    println!("========================================");
    println!(" Done! Exiting cleanly.");
    0
}
