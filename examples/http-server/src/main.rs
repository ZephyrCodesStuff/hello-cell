//! PS3 HTTP Server Example using SPRX Networking & Sockets

#![no_std]
#![no_main]

use ps3::alloc::format;
use ps3::allocator;
use ps3::net;
use ps3::println;
use ps3::sys::syscalls;

#[no_mangle]
pub extern "C" fn rust_main() -> i32 {
    println!("========================================");
    println!(" PS3 HTTP Server via Rust SDK");
    println!("========================================");

    // Initialize networking subsystem via SPRX stubs
    println!(" [NET] Initializing PS3 Network Subsystem via SPRX Stubs...");
    if let Err(e) = net::init() {
        println!(" [NET] Network init returned error: {:#X}", e as u32);
        return 1;
    }
    println!(" [NET] SUCCESS! Network stack initialized.");

    // Bind TCP listener on 0.0.0.0:8080
    println!(" [HTTP] Binding TCP listener on 0.0.0.0:8080...");
    let listener = match net::TcpListener::bind([0, 0, 0, 0], 8080) {
        Ok(l) => {
            println!(" [HTTP] Server listening at http://0.0.0.0:8080 !");
            println!(" [HTTP] Ready to receive HTTP GET requests from your PC/browser...");
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
                        let body = format!(
                            "Hello from PlayStation 3 (CellOS Rust)!\n\n\
                            Request Count: {}\n\
                            Heap Statistics:\n\
                            - Active Allocations: {}\n\
                            - Active Bytes:       {} bytes\n\
                            - Claimed Total:      {} MB\n",
                            request_count,
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
