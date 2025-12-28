use crate::memory::MemoryError::{AlignmentError, LockedAllocator, MappingError};
use crate::memory::paging::bitmap_allocator::BitmapAllocator;
use crate::memory::{
    KERNEL_PAGE_TABLE, KERNEL_VIRTUAL_PAGE_ALLOCATOR, MemoryError, PAGE_SIZE,
    PHYSICAL_FRAME_ALLOCATOR,
};

use crate::{kprintln, return_if_none};
use alloc::vec::Vec;
use core::ops::Add;
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, Translate};
use x86_64::{PhysAddr, VirtAddr};

pub struct VirtualPageAllocator {
    pub(crate) allocator: BitmapAllocator,
    offset: u64,
}

impl VirtualPageAllocator {
    pub(crate) fn new(
        address: VirtAddr,
        size: usize,
        offset: VirtAddr,
    ) -> Result<VirtualPageAllocator, MemoryError> {
        let allocator = BitmapAllocator::new(address.as_mut_ptr(), size, true)?;

        kprintln!("Created Virtual Page Allocator at {:?}", address);

        Ok(VirtualPageAllocator {
            allocator,
            offset: offset.as_u64(),
        })
    }

    pub(crate) fn alloc(&mut self, pages: u64) -> Result<VirtAddr, MemoryError> {
        self.allocator
            .alloc(pages)
            .map(|page| VirtAddr::new(self.offset + page * PAGE_SIZE as u64))
    }

    fn free(&mut self, addr: VirtAddr) -> Result<(), MemoryError> {
        let offset = (addr.as_u64() - self.offset) / PAGE_SIZE as u64;
        self.allocator.free(offset)
    }
}

pub fn map(frames: Vec<PhysFrame>, flags: PageTableFlags) -> Result<VirtAddr, MemoryError> {
    let frame_count = frames.len();

    let virt_addr = KERNEL_VIRTUAL_PAGE_ALLOCATOR
        .lock()
        .get_mut()
        .unwrap()
        .alloc(frame_count as u64)?;

    let mut allocator = PHYSICAL_FRAME_ALLOCATOR.lock();

    let allocator = return_if_none!(allocator.get_mut(), LockedAllocator);

    let mut page_table = KERNEL_PAGE_TABLE.lock();
    let page_table = page_table.get_mut().unwrap();

    for (offset, frame) in frames.into_iter().enumerate() {
        let page = match Page::from_start_address(virt_addr.add((PAGE_SIZE * offset) as u64)) {
            Err(_) => return Err(AlignmentError),
            Ok(page) => page,
        };

        unsafe {
            match page_table.map_to(page, frame, flags, allocator) {
                Ok(flusher) => flusher.flush(),
                Err(_) => return Err(MappingError),
            }
        }
    }

    Ok(virt_addr)
}

pub fn unmap(base_addr: VirtAddr, pages: u64) -> Result<(), MemoryError> {
    let mut page_table = KERNEL_PAGE_TABLE.lock();
    let page_table = page_table.get_mut().unwrap();

    let mut allocator = KERNEL_VIRTUAL_PAGE_ALLOCATOR.lock();
    let allocator = allocator.get_mut().unwrap();

    for page_index in 0..pages {
        let page: Page<Size4KiB> =
            match Page::from_start_address(base_addr.add(PAGE_SIZE as u64 * page_index)) {
                Err(_) => return Err(AlignmentError),
                Ok(page) => page,
            };

        match page_table.unmap(page) {
            Ok(result) => {
                result.1.flush();
                allocator.free(base_addr.add(page_index * PAGE_SIZE as u64))?
            }
            Err(_) => return Err(MappingError),
        }
    }
    Ok(())
}

pub fn translate_to_phys(addr: VirtAddr) -> Option<PhysAddr> {
    KERNEL_PAGE_TABLE.lock().get().unwrap().translate_addr(addr)
}
