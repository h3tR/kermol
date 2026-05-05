use crate::memory::MemoryError;
use crate::memory::paging::page_table::RecursivePageTable;
use crate::memory::paging::{
    BitmapFrameAllocator, FrameAllocator, PagingError, VirtualMemoryAllocator,
};
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};
use crate::memory::MemoryError::PageNotPresent;

pub struct KernelPagingController {
    pub(crate) rec_page_table: RecursivePageTable,
    pub(crate) frame_allocator: BitmapFrameAllocator,
    pub(crate) virt_mem_allocator: VirtualMemoryAllocator,
}

unsafe impl Send for KernelPagingController {}

impl KernelPagingController {
    fn map(
        &mut self,
        from: PhysAddr,
        to: VirtAddr,
        page_count: usize,
        flags: PageTableFlags,
    ) -> Result<(), MemoryError> {
        self.rec_page_table
            .map_contiguous(page_count, from, to, flags, &mut self.frame_allocator)
            .map_err(|e| MemoryError::from(e))
    }

    fn unmap(&mut self, addr: VirtAddr, page_count: usize) -> Result<(), MemoryError> {
        self.rec_page_table
            .unmap_contiguous(page_count, addr, &mut self.frame_allocator)
            .map_err(|e| MemoryError::from(e))
    }

    fn alloc_and_map(
        &mut self,
        pages: usize,
        flags: PageTableFlags,
    ) -> Result<VirtAddr, MemoryError> {
        let phys = self.frame_allocator.alloc_contiguous(pages)?;
        let virt = self.virt_mem_allocator.alloc_contiguous(pages)?;
        self.map(phys, virt, pages, flags)?;
        Ok(virt)
    }

    fn free_and_unmap(&mut self, addr: VirtAddr, page_count: usize) -> Result<(), MemoryError> {
        self.virt_mem_allocator.free(addr, page_count)?;
        let phys = self.rec_page_table.translate(addr).ok_or(PageNotPresent)?;
        self.frame_allocator.free_contiguous(phys, page_count)?;
        self.unmap(addr, page_count)?;

        Ok(())
    }
}
