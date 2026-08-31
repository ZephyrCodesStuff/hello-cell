//! Low-level C ABI types, structs, and constants for PlayStation 3 CellOS LV2.

// -----------------------------------------------------------------------------
// Memory Constants
// -----------------------------------------------------------------------------
pub const SYS_MEMORY_PAGE_SIZE_64K: u64 = 0x200;
pub const SYS_MEMORY_PAGE_SIZE_1M: u64 = 0x400;

// -----------------------------------------------------------------------------
// System Module IDs
// -----------------------------------------------------------------------------
pub const SYSMODULE_NET: i32 = 0x0000;
pub const SYSMODULE_NETCTL: i32 = 0x0014;

// -----------------------------------------------------------------------------
// Sockets & Network Constants
// -----------------------------------------------------------------------------
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
pub struct CellNetCtlInfo {
    pub ip_address: [u8; 16],
}
