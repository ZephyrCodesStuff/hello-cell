//! Global Heap Allocator for PlayStation 3 using Talc 5.
//!
//! Provides high-performance O(1) allocation and deallocation, backed by
//! dynamic heap expansion via PS3 LV2 `sys_memory_allocate`.

use crate::sys::{sys_memory_allocate, SYS_MEMORY_PAGE_SIZE_64K};
use core::alloc::Layout;
use talc::base::binning::Binning;
use talc::base::Talc;
use talc::source::Source;
use talc::TalcLock;

#[derive(Debug)]
pub struct Ps3Source;

unsafe impl Source for Ps3Source {
    fn acquire<B: Binning>(talc: &mut Talc<Self, B>, layout: Layout) -> Result<(), ()> {
        let min_size = layout.size() + layout.align() + 4096;
        let chunk_size = min_size.max(4 * 1024 * 1024); // at least 4 MB
        let aligned_size = (chunk_size + 0xFFFF) & !0xFFFF;

        match unsafe { sys_memory_allocate(aligned_size, SYS_MEMORY_PAGE_SIZE_64K) } {
            Ok(ptr) => unsafe {
                let _ = talc.claim(ptr, aligned_size);
                Ok(())
            },
            Err(_) => Err(()),
        }
    }
}

use core::sync::atomic::{AtomicBool, Ordering};
use lock_api::RawMutex;

pub struct Ps3RawMutex(AtomicBool);

unsafe impl RawMutex for Ps3RawMutex {
    const INIT: Self = Self(AtomicBool::new(false));
    type GuardMarker = lock_api::GuardSend;

    fn lock(&self) {
        while self.0.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
    }

    fn try_lock(&self) -> bool {
        !self.0.swap(true, Ordering::Acquire)
    }

    unsafe fn unlock(&self) {
        self.0.store(false, Ordering::Release);
    }
}

#[global_allocator]
pub static ALLOCATOR: TalcLock<Ps3RawMutex, Ps3Source> = TalcLock::new(Ps3Source);

#[derive(Debug, Clone, Copy)]
pub struct HeapStats {
    pub active_allocations: usize,
    pub active_bytes: usize,
    pub total_allocations: u64,
    pub total_allocated_bytes: u64,
    pub claimed_bytes: usize,
}

pub fn get_heap_stats() -> HeapStats {
    let talc = ALLOCATOR.lock();
    let counters = talc.counters();
    HeapStats {
        active_allocations: counters.allocation_count,
        active_bytes: counters.allocated_bytes,
        total_allocations: counters.total_allocation_count,
        total_allocated_bytes: counters.total_allocated_bytes,
        claimed_bytes: counters.claimed_bytes,
    }
}
