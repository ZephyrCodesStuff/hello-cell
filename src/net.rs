//! PlayStation 3 Networking Library & Sockets
//!
//! Implements native networking using FNID dynamic linking.

use alloc::alloc::{alloc, dealloc};
use core::alloc::Layout;

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------
pub const SYSMODULE_NET: i32 = 0x0000;
pub const SYSMODULE_NETCTL: i32 = 0x0014;

pub const AF_INET: u8 = 2;
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;
pub const IPPROTO_TCP: i32 = 6;
pub const IPPROTO_UDP: i32 = 17;
pub const SHUT_RDWR: i32 = 2;

pub const CELL_NET_CTL_STATE_DISCONNECTED: i32 = 0;
pub const CELL_NET_CTL_STATE_CONNECTING: i32 = 1;
pub const CELL_NET_CTL_STATE_IPOBTAINING: i32 = 2;
pub const CELL_NET_CTL_STATE_IPOBTAINED: i32 = 3;

pub const CELL_NET_CTL_INFO_IP_ADDRESS: i32 = 16;

// -----------------------------------------------------------------------------
// C-ABI Structures
// -----------------------------------------------------------------------------
#[repr(C)]
pub struct NetInitParam {
    pub memory: u32,
    pub memory_size: u32,
    pub flags: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SockAddrIn {
    pub sin_len: u8,
    pub sin_family: u8,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

impl SockAddrIn {
    pub fn new(ip: [u8; 4], port: u16) -> Self {
        let sin_addr = u32::from_be_bytes(ip);
        Self {
            sin_len: core::mem::size_of::<Self>() as u8,
            sin_family: AF_INET,
            sin_port: port.to_be(),
            sin_addr,
            sin_zero: [0; 8],
        }
    }
}

#[repr(C)]
pub struct SockAddr {
    pub sa_len: u8,
    pub sa_family: u8,
    pub sa_data: [u8; 14],
}

#[repr(C)]
pub union CellNetCtlInfo {
    pub ip_address: [u8; 16],
    pub raw: [u8; 512],
}

// -----------------------------------------------------------------------------
// SPRX FFI Imports (Resolved via FNIDs in src/sprx.s)
// -----------------------------------------------------------------------------
extern "C" {
    pub fn sysModuleLoad(id: i32) -> i32;
    pub fn sysModuleUnload(id: i32) -> i32;
    pub fn sysModuleIsLoaded(id: i32) -> i32;

    pub fn netInitializeNetworkEx(param: *const NetInitParam) -> i32;
    pub fn netFinalizeNetwork() -> i32;

    pub fn netSocket(domain: i32, socket_type: i32, protocol: i32) -> i32;
    pub fn netConnect(socket: i32, addr: *const SockAddr, addrlen: u32) -> i32;
    pub fn netBind(socket: i32, addr: *const SockAddr, addrlen: u32) -> i32;
    pub fn netListen(socket: i32, backlog: i32) -> i32;
    pub fn netAccept(socket: i32, addr: *mut SockAddr, addrlen: *mut u32) -> i32;
    pub fn netSend(socket: i32, buf: *const u8, len: usize, flags: i32) -> isize;
    pub fn netSendTo(
        socket: i32,
        buf: *const u8,
        len: usize,
        flags: i32,
        dest_addr: *const SockAddr,
        dest_len: u32,
    ) -> isize;
    pub fn netRecv(socket: i32, buf: *mut u8, len: usize, flags: i32) -> isize;
    pub fn netRecvFrom(
        socket: i32,
        buf: *mut u8,
        len: usize,
        flags: i32,
        from_addr: *mut SockAddr,
        from_len: *mut u32,
    ) -> isize;
    pub fn netShutdown(socket: i32, how: i32) -> i32;
    pub fn netClose(socket: i32) -> i32;

    pub fn cellNetCtlInit() -> i32;
    pub fn cellNetCtlTerm();
    pub fn cellNetCtlGetState(state: *mut i32) -> i32;
    pub fn cellNetCtlGetInfo(code: i32, info: *mut CellNetCtlInfo) -> i32;
}

// -----------------------------------------------------------------------------
// Global Network Lifecycle
// -----------------------------------------------------------------------------
static mut NET_BUFFER: *mut u8 = core::ptr::null_mut();
const NET_BUFFER_SIZE: usize = 128 * 1024; // 128 KB for network stack

/// Initializes the PS3 network subsystem and network control (cellNetCtl).
pub fn init() -> Result<(), i32> {
    unsafe {
        if !NET_BUFFER.is_null() {
            return Ok(());
        }

        // 1. Load sys_net.sprx module
        crate::println!(" [NET] Loading CELL_SYSMODULE_NET (0x0000)...");
        let res = sysModuleLoad(SYSMODULE_NET);
        if res < 0 && res != -0x7FFEDFFF
        /* 0x80012001: SYSMODULE_ERR_DUPLICATE */
        {
            return Err(res);
        }

        // 2. Allocate 128 KB buffer using our Talc allocator
        let layout = Layout::from_size_align_unchecked(NET_BUFFER_SIZE, 64);
        let ptr = alloc(layout);
        if ptr.is_null() {
            return Err(-1);
        }
        core::ptr::write_bytes(ptr, 0, NET_BUFFER_SIZE);
        NET_BUFFER = ptr;

        // 3. Initialize PS3 network stack
        crate::println!(" [NET] Initializing libnet with 128KB buffer...");
        let params = NetInitParam {
            memory: ptr as usize as u32,
            memory_size: NET_BUFFER_SIZE as u32,
            flags: 0,
        };

        let net_res = netInitializeNetworkEx(&params);
        if net_res != 0 {
            dealloc(ptr, layout);
            NET_BUFFER = core::ptr::null_mut();
            return Err(net_res);
        }

        // 4. Load libnetctl.sprx module
        crate::println!(" [NET] Loading CELL_SYSMODULE_NETCTL (0x0014)...");
        let ctl_load = sysModuleLoad(SYSMODULE_NETCTL);
        if ctl_load < 0 && ctl_load != -0x7FFEDFFF {
            crate::println!(" [NET] sysModuleLoad(NETCTL) error: {:#X}", ctl_load as u32);
        }

        // 5. Initialize cellNetCtl
        crate::println!(" [NET] Initializing cellNetCtl...");
        let ctl_init = cellNetCtlInit();
        crate::println!(" [NET] cellNetCtlInit result: {:#X}", ctl_init as u32);

        // 6. Wait for IP address
        crate::println!(" [NET] Waiting for network connection (IPObtained)...");
        let mut state = -1i32;
        for i in 0..15 {
            let s_res = cellNetCtlGetState(&mut state);
            crate::println!(
                " [NET] [poll {}] cellNetCtlGetState -> res: {:#X}, state: {}",
                i,
                s_res as u32,
                state
            );
            if s_res == 0 && state == CELL_NET_CTL_STATE_IPOBTAINED {
                crate::println!(" [NET] Network connection active (State=IPObtained)!");
                break;
            }
            crate::syscalls::sys_timer_usleep(250_000); // 250ms
        }

        // 7. Obtain and display PS3 IP Address
        let mut info = CellNetCtlInfo { raw: [0; 512] };
        let info_res = cellNetCtlGetInfo(CELL_NET_CTL_INFO_IP_ADDRESS, &mut info);
        crate::println!(" [NET] cellNetCtlGetInfo result: {:#X}", info_res as u32);
        if info_res == 0 {
            let ip_str = core::str::from_utf8(&info.ip_address).unwrap_or("unknown");
            let clean_ip = ip_str.trim_matches(char::from(0));
            crate::println!(" [NET] PS3 IP Address: {}", clean_ip);
        }

        Ok(())
    }
}

/// Shuts down the PS3 network subsystem.
pub fn deinit() {
    unsafe {
        if NET_BUFFER.is_null() {
            return;
        }
        cellNetCtlTerm();
        sysModuleUnload(SYSMODULE_NETCTL);
        netFinalizeNetwork();
        let layout = Layout::from_size_align_unchecked(NET_BUFFER_SIZE, 64);
        dealloc(NET_BUFFER, layout);
        NET_BUFFER = core::ptr::null_mut();
        sysModuleUnload(SYSMODULE_NET);
    }
}

// -----------------------------------------------------------------------------
// High-Level Socket Abstractions
// -----------------------------------------------------------------------------

/// A UDP socket for sending and receiving datagrams.
pub struct UdpSocket {
    fd: i32,
}

impl UdpSocket {
    pub fn bind(ip: [u8; 4], port: u16) -> Result<Self, i32> {
        let fd = unsafe { netSocket(AF_INET as i32, SOCK_DGRAM, 0) };
        if fd < 0 {
            return Err(fd);
        }

        let addr = SockAddrIn::new(ip, port);
        let res = unsafe {
            netBind(
                fd,
                &addr as *const _ as *const SockAddr,
                core::mem::size_of::<SockAddrIn>() as u32,
            )
        };

        if res < 0 {
            unsafe {
                netClose(fd);
            }
            return Err(res);
        }

        Ok(Self { fd })
    }

    pub fn send_to(&self, buf: &[u8], ip: [u8; 4], port: u16) -> Result<usize, i32> {
        let addr = SockAddrIn::new(ip, port);
        let sent = unsafe {
            netSendTo(
                self.fd,
                buf.as_ptr(),
                buf.len(),
                0,
                &addr as *const _ as *const SockAddr,
                core::mem::size_of::<SockAddrIn>() as u32,
            )
        };

        if sent < 0 {
            Err(sent as i32)
        } else {
            Ok(sent as usize)
        }
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                netClose(self.fd);
            }
        }
    }
}

/// A TCP stream connection.
pub struct TcpStream {
    fd: i32,
}

impl TcpStream {
    pub fn connect(ip: [u8; 4], port: u16) -> Result<Self, i32> {
        let fd = unsafe { netSocket(AF_INET as i32, SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(fd);
        }

        let addr = SockAddrIn::new(ip, port);
        let res = unsafe {
            netConnect(
                fd,
                &addr as *const _ as *const SockAddr,
                core::mem::size_of::<SockAddrIn>() as u32,
            )
        };

        if res < 0 {
            unsafe {
                netClose(fd);
            }
            return Err(res);
        }

        Ok(Self { fd })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, i32> {
        let sent = unsafe { netSend(self.fd, buf.as_ptr(), buf.len(), 0) };
        if sent < 0 {
            Err(sent as i32)
        } else {
            Ok(sent as usize)
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        let received = unsafe { netRecv(self.fd, buf.as_mut_ptr(), buf.len(), 0) };
        if received < 0 {
            Err(received as i32)
        } else {
            Ok(received as usize)
        }
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                netShutdown(self.fd, SHUT_RDWR);
                netClose(self.fd);
            }
        }
    }
}

/// A TCP listener socket for accepting incoming connections.
pub struct TcpListener {
    fd: i32,
}

impl TcpListener {
    pub fn bind(ip: [u8; 4], port: u16) -> Result<Self, i32> {
        let fd = unsafe { netSocket(AF_INET as i32, SOCK_STREAM, 0) };
        if fd < 0 {
            crate::println!(
                " [NET] netSocket(AF_INET=2, SOCK_STREAM=1, proto=0) failed: {:#X}",
                fd as u32
            );
            return Err(fd);
        }

        let addr = SockAddrIn::new(ip, port);
        let res = unsafe {
            netBind(
                fd,
                &addr as *const _ as *const SockAddr,
                core::mem::size_of::<SockAddrIn>() as u32,
            )
        };

        if res < 0 {
            crate::println!(" [NET] netBind failed on port {}: {:#X}", port, res as u32);
            unsafe {
                netClose(fd);
            }
            return Err(res);
        }

        let listen_res = unsafe { netListen(fd, 8) };
        if listen_res < 0 {
            crate::println!(" [NET] netListen failed: {:#X}", listen_res as u32);
            unsafe {
                netClose(fd);
            }
            return Err(listen_res);
        }

        Ok(Self { fd })
    }

    pub fn accept(&self) -> Result<(TcpStream, SockAddrIn), i32> {
        let mut client_addr = SockAddrIn::new([0, 0, 0, 0], 0);
        let mut addr_len = core::mem::size_of::<SockAddrIn>() as u32;

        let client_fd = unsafe {
            netAccept(
                self.fd,
                &mut client_addr as *mut _ as *mut SockAddr,
                &mut addr_len,
            )
        };

        if client_fd < 0 {
            Err(client_fd)
        } else {
            Ok((TcpStream { fd: client_fd }, client_addr))
        }
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                netClose(self.fd);
            }
        }
    }
}
