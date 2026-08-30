//! PlayStation 3 Barebones Rust Runtime
//!
//! Provides core language runtime, heap allocation, TTY output, and LV2 syscalls.

#![no_std]
#![no_main]

extern crate alloc;

pub mod allocator;
pub mod entry;
pub mod io;
pub mod mem;
pub mod net;
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

    // -------------------------------------------------------------------------
    // Memory & PRX Subsystem Probing
    // -------------------------------------------------------------------------
    println!("----------------------------------------");
    println!(" Probing PS3 Memory & PRX Subsystem...");

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

    // Test SPRX Dynamic Stubs via cellSysmodule & sys_net
    println!(" [NET] Initializing PS3 Network Subsystem via SPRX Stubs...");
    if let Err(e) = net::init() {
        println!(" [NET] Network init returned error: {:#X}", e as u32);
        return 1;
    }
    println!(" [NET] SUCCESS! Network stack initialized via SPRX stubs.");
    println!("----------------------------------------");

    // -------------------------------------------------------------------------
    // HTTP Server on Port 8080
    // -------------------------------------------------------------------------
    println!(" [HTTP] Binding TCP listener on 0.0.0.0:8080...");
    let listener = match net::TcpListener::bind([0, 0, 0, 0], 8080) {
        Ok(l) => {
            println!(" [HTTP] Server listening at http://0.0.0.0:8080 !");
            println!(" [HTTP] Ready to receive HTTP GET requests from your PC...");
            l
        }
        Err(e) => {
            println!(" [HTTP] Failed to bind TCP listener: {:#X}", e as u32);
            return 1;
        }
    };

    let mut request_count = 0u64;
    loop {
        println!(" [HTTP] Waiting for incoming connection...");
        match listener.accept() {
            Ok((mut stream, client_addr)) => {
                request_count += 1;
                let ip_bytes = client_addr.sin_addr.to_be_bytes();
                let port = u16::from_be(client_addr.sin_port);
                println!(
                    " [HTTP] Connection #{} accepted from {}.{}.{}.{}:{}",
                    request_count, ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3], port
                );

                let mut req_buf = [0u8; 1024];
                match stream.read(&mut req_buf) {
                    Ok(n) if n > 0 => {
                        let req_str =
                            core::str::from_utf8(&req_buf[..n]).unwrap_or("<binary data>");
                        let first_line = req_str.lines().next().unwrap_or("");
                        println!(" [HTTP] Request: '{}'", first_line);

                        let heap_stats = allocator::get_heap_stats();
                        let body = format!("Hello, Cell!\n\nStats:\n- Active Allocs: {}\n- Active Bytes: {} bytes\n- Claimed Total: {} MB\n",
                            heap_stats.active_allocations,
                            heap_stats.active_bytes,
                            heap_stats.claimed_bytes / (1024 * 1024)
                        );

                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                            Content-Type: text/plain; charset=utf-8\r\n\
                            Content-Length: {}\r\n\
                            Connection: close\r\n\
                            Server: PS3-CellOS-Rust/1.0\r\n\
                            \r\n\
                            {}",
                            body.len(),
                            body
                        );

                        match stream.write(response.as_bytes()) {
                            Ok(sent) => println!(" [HTTP] Sent {} bytes response (200 OK)!", sent),
                            Err(e) => println!(" [HTTP] Send error: {:#X}", e as u32),
                        }
                    }
                    Ok(_) => {
                        println!(" [HTTP] Connection closed by client (0 bytes read)");
                    }
                    Err(e) => {
                        println!(" [HTTP] Read error: {:#X}", e as u32);
                    }
                }
            }
            Err(e) => {
                println!(" [HTTP] Accept error: {:#X}", e as u32);
                syscalls::sys_timer_usleep(500_000);
            }
        }
    }
}
