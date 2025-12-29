use crate::memory::paging::bitmap_allocator::BitmapAllocator;
use crate::memory::{MemoryError, PAGE_SIZE};

use crate::memory::paging::frame_allocation::LinearFrameAllocator;
use crate::memory::paging::page_table::RecursivePageTable;
use core::ops::Add;
use x86_64::VirtAddr;

pub(super) const PAGE_ALLOCATOR_START: VirtAddr = VirtAddr::new(0xFFFF_E000_0000_0000);
pub struct VirtualMemoryAllocator(BitmapAllocator);

impl VirtualMemoryAllocator {
    pub(crate) fn new(
        size: usize,
        frame_allocator: &mut LinearFrameAllocator,
        page_table: &mut RecursivePageTable,
    ) -> Result<VirtualMemoryAllocator, MemoryError> {
        let bitmap = BitmapAllocator::new(size, true, frame_allocator, page_table)?;

        Ok(VirtualMemoryAllocator(bitmap))
    }

    pub fn alloc_contiguous(&mut self, pages: usize) -> Result<VirtAddr, MemoryError> {
        self.0
            .alloc(pages as u64)
            .map(|page| PAGE_ALLOCATOR_START.add(page * PAGE_SIZE as u64))
    }

    //TODO: think about making alloc_non_contiguous in some way and if that is worth

    pub fn free(&mut self, addr: VirtAddr, pages: usize) -> Result<(), MemoryError> {
        let offset = (addr.as_u64() - PAGE_ALLOCATOR_START.as_u64()) / PAGE_SIZE as u64;
        self.0.free_range(&(offset..(offset + pages as u64)))
    }
}
