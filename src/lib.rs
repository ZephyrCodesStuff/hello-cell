//! PlayStation 3 Barebones Rust Runtime
//!
//! Provides core language runtime, heap allocation, TTY output, and LV2 syscalls.

#![no_std]

extern crate alloc;

pub mod allocator;
pub mod entry;
pub mod io;
pub mod mem;
pub mod panic;
pub mod syscalls;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[no_mangle]
pub extern "C" fn rust_main() -> i32 {
    println!("========================================");
    println!(" Hello PlayStation 3 from barebones Rust!");
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
    let formatted_str: String = format!("Formatted string dynamically: 0x{:X}", 0xCAFEBABE_u32);
    println!("{}", formatted_str.as_str());

    let stats_before = allocator::get_heap_stats();
    println!("--- Heap Stats Before Temp Allocations ---");
    println!("  Active Allocs: {}", stats_before.active_allocations);
    println!("  Active Bytes:  {} bytes", stats_before.active_bytes);
    println!(
        "  Claimed Total: {} MB",
        stats_before.claimed_bytes / (1024 * 1024)
    );

    // Test Deallocation & Reallocation (Talc recycling test)
    println!("Allocating a 64KB vector (8000 u64 elements)...");
    {
        let mut big_vec: Vec<u64> = Vec::with_capacity(8000);
        for j in 0..8000 {
            big_vec.push(j);
        }
        let stats_during = allocator::get_heap_stats();
        println!("--- Heap Stats WHILE Vector is Live ---");
        println!("  Active Allocs: {}", stats_during.active_allocations);
        println!(
            "  Active Bytes:  {} bytes (+{})",
            stats_during.active_bytes,
            stats_during.active_bytes - stats_before.active_bytes
        );
        println!("  Vector capacity in bytes: {}", big_vec.capacity() * 8);

        // Vector goes out of scope here and deallocates
    }

    let stats_after = allocator::get_heap_stats();
    println!("--- Heap Stats AFTER Vector Dropped (Deallocated!) ---");
    println!("  Active Allocs: {}", stats_after.active_allocations);
    println!(
        "  Active Bytes:  {} bytes (Freed back to Talc!)",
        stats_after.active_bytes
    );
    println!("  Total Lifetime Allocs: {}", stats_after.total_allocations);

    println!("All allocations, deallocations, and formatting succeeded!");

    // Heartbeat loop
    let mut tick = 0u64;
    loop {
        syscalls::sys_timer_usleep(1_000_000); // 1 second
        tick += 1;
        println!("Heartbeat tick: {}", tick);
    }
}
