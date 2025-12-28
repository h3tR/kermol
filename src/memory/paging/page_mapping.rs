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

pub(super) const PAGE_ALLOCATOR_START: VirtAddr = VirtAddr::new(0xFFFF_E000_0000_0000);
pub struct VirtualPageAllocator(BitmapAllocator);

impl VirtualPageAllocator {
    pub(crate) fn new(address: VirtAddr, size: usize) -> Result<VirtualPageAllocator, MemoryError> {
        Ok(VirtualPageAllocator(BitmapAllocator::new(
            address.as_mut_ptr(),
            size,
            true,
        )?))
    }

    pub(crate) fn alloc(&mut self, pages: u64) -> Result<VirtAddr, MemoryError> {
        self.0
            .alloc(pages)
            .map(|page| PAGE_ALLOCATOR_START.add(page * PAGE_SIZE as u64))
    }

    fn free(&mut self, addr: VirtAddr) -> Result<(), MemoryError> {
        let offset = (addr.as_u64() - PAGE_ALLOCATOR_START.as_u64()) / PAGE_SIZE as u64;
        self.0.free(offset)
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
