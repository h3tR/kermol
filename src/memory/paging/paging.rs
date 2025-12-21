use crate::memory::paging::bitmap_allocator::BitmapAllocator;
use crate::memory::paging::frame_allocation::LinearFrameAllocator;
use crate::memory::paging::{get_total_frames, FRAME_ALLOCATOR, PAGE_SIZE, PAGE_TABLE, VIRTUAL_PAGE_ALLOCATOR};
use crate::memory::MemoryError::{AlignmentError, LockedAllocator, MappingError};
use crate::memory::MemoryError;
use crate::{kprintln, return_if_none};
use alloc::vec::Vec;
use core::ops::Add;
use limine_protocol_for_rust::requests::memory_map::MemoryMapResponse;
use x86_64::structures::paging::{
    Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, Translate,
};
use x86_64::{PhysAddr, VirtAddr};

const VIRTUAL_MEMORY_BASE: u64 = 0xFFFF_8000_0000_0000;

pub struct VirtualPageAllocator(BitmapAllocator);

impl VirtualPageAllocator {
    pub(crate) fn new(
        memory_map: &'static MemoryMapResponse,
        init_allocator: &mut LinearFrameAllocator,
    ) -> Result<VirtualPageAllocator, MemoryError> {
        //calculate total frames in memory
        let total_frames = get_total_frames(memory_map);
        kprintln!("Total Allocatable frames: {}", total_frames);

        let bitmap_size = (total_frames / 8) as usize;
        kprintln!("Calculated bitmap size: {}", bitmap_size);

        for frame_index in 0..bitmap_size {
            let frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(
                (total_frames / 2 + frame_index as u64) * PAGE_SIZE as u64,
            ))
            .map_err(|_| AlignmentError)?;

            init_allocator.identity_map(frame)?;
        }
        
        kprintln!("VPA Bitmap allocated");

        let addr = VirtAddr::new(total_frames / 2 * PAGE_SIZE as u64);

        let allocator = BitmapAllocator::new(addr.as_mut_ptr(), total_frames as usize, 0);

        kprintln!("Created Virtual Page Allocator at {:?}", addr);

        Ok(VirtualPageAllocator(allocator))
    }

    pub(crate) fn alloc(&mut self, pages: u64) -> Result<VirtAddr, MemoryError> {
        self.0
            .alloc(pages)
            .map(|page| VirtAddr::new(VIRTUAL_MEMORY_BASE + page * PAGE_SIZE as u64))
    }

    fn free(&mut self, addr: VirtAddr) -> Result<(), MemoryError> {
        let offset = (addr.as_u64() - VIRTUAL_MEMORY_BASE) / PAGE_SIZE as u64;
        self.0.free(offset)
    }
}

pub fn map(
    frames: Vec<PhysFrame>,
    flags: PageTableFlags,
) -> Result<VirtAddr, MemoryError> {
    let frame_count = frames.len();

    let virt_addr = VIRTUAL_PAGE_ALLOCATOR
        .lock()
        .get_mut()
        .unwrap()
        .alloc(frame_count as u64)?;

    let mut allocator = FRAME_ALLOCATOR.lock();
    
    let allocator = return_if_none!(allocator.get_mut(), LockedAllocator);

    let mut page_table = PAGE_TABLE.lock();
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
    let mut page_table = PAGE_TABLE.lock();
    let page_table = page_table.get_mut().unwrap();

    let mut allocator = VIRTUAL_PAGE_ALLOCATOR.lock();
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
    PAGE_TABLE.lock().get().unwrap().translate_addr(addr)
}
