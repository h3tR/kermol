use limine_protocol_for_rust::requests::memory_map::MemoryMapResponse;
use spin::{Mutex, Once};
use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, PageTableFlags, PhysFrame, Size4KiB};
use crate::memory::MemoryError;
use crate::memory::MemoryError::MappingError;
use crate::memory::paging::frame_allocation::BitmapFrameAllocator;
pub(crate) use crate::memory::paging::page_mapping::VirtualPageAllocator;
use crate::util::KIBIBYTE;

pub(super) mod bitmap_allocator;
pub(super) mod frame_allocation;
pub(super) mod page_mapping;

pub const PAGE_SIZE: usize = 4 * KIBIBYTE;

pub static FRAME_ALLOCATOR: Mutex<Once<BitmapFrameAllocator>> = Mutex::new(Once::new());

pub(super) static VIRTUAL_PAGE_ALLOCATOR: Mutex<Once<VirtualPageAllocator>> =
    Mutex::new(Once::new());

pub(super) static PAGE_TABLE: Mutex<Once<OffsetPageTable<'static>>> = Mutex::new(Once::new());

///downward growing linear allocator for initialization of the bitmap frame allocators
pub struct LinearFrameAllocator {
    allocated_frames: u64,
    base_frame: u64,
}

impl LinearFrameAllocator {
    pub fn new(memory_map: &'static MemoryMapResponse) -> Self {
        let total_frames = memory_map
            .get_entries()
            .iter()
            .cloned()
            .map(|r| r.base..(r.base + r.length))
            .flat_map(|r| r.step_by(PAGE_SIZE))
            .count() as u64;
        Self {
            allocated_frames: 0,
            base_frame: total_frames / 2 - 1,
        }
    }

    pub fn identity_map(&mut self, frame: PhysFrame<Size4KiB>) -> Result<(), MemoryError> {
        unsafe {
            PAGE_TABLE
                .lock()
                .get_mut()
                .unwrap()
                .identity_map(
                    frame,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_EXECUTE,
                    self,
                )
                .map_err(|_| MappingError)?
                .flush();
        }
        Ok(())
    }
}

unsafe impl FrameAllocator<Size4KiB> for LinearFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocated_frames += 1;
        Some(PhysFrame::containing_address(PhysAddr::new(
            (self.base_frame - self.allocated_frames) * PAGE_SIZE as u64,
        )))
    }
}


fn get_total_frames(memory_map: &'static MemoryMapResponse) -> u64 {
    memory_map
        .get_entries()
        .iter()
        .map(|r| r.base..(r.base + r.length))
        .flat_map(|r| r.step_by(PAGE_SIZE))
        .count() as u64
}