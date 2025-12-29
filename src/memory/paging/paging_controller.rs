use crate::memory::MemoryError;
use crate::memory::MemoryError::TODOError;
use crate::memory::paging::page_table::RecursivePageTable;
use crate::memory::paging::{BitmapFrameAllocator, FrameAllocator, VirtualMemoryAllocator};
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};

pub struct KernelPagingController {
    pub(crate) k_page_table: RecursivePageTable,
    pub(crate) k_frame_allocator: BitmapFrameAllocator,
    pub(super) k_virt_mem_allocator: VirtualMemoryAllocator,
}

unsafe impl Send for KernelPagingController {}

impl KernelPagingController {
    fn map(&mut self, from: PhysAddr, to: VirtAddr, page_count: usize, flags: PageTableFlags) -> Result<(), MemoryError> {
        self.k_page_table
            .map_contiguous(page_count, from, to, flags, &mut self.k_frame_allocator)
    }

    fn unmap(&mut self, addr: VirtAddr, page_count: usize) -> Result<(), MemoryError> {
        self.k_page_table
            .unmap(page_count, addr, &mut self.k_frame_allocator)
    }

    fn alloc_and_map(
        &mut self,
        pages: usize,
        flags: PageTableFlags,
    ) -> Result<VirtAddr, MemoryError> {
        let phys = self.k_frame_allocator.alloc_contiguous(pages)?;
        let virt = self.k_virt_mem_allocator.alloc_contiguous(pages)?;
        self.map(phys, virt, pages, flags)?;
        Ok(virt)
    }

    fn free_and_unmap(&mut self, addr: VirtAddr, page_count: usize) -> Result<(), MemoryError> {
        self.k_virt_mem_allocator.free(addr, page_count)?;
        let phys = self.k_page_table.translate(addr).ok_or(TODOError)?;
        self.k_frame_allocator.free_contiguous(phys, page_count)?;
        self.unmap(addr, page_count)?;

        Ok(())
    }
}
