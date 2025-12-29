use crate::memory::paging::bitmap_allocator::BitmapAllocator;
use crate::memory::paging::page_table::RecursivePageTable;
use crate::memory::{MemoryError, PAGE_SIZE};
use crate::{kprintln, serial_println};
use core::ops::{Add, AddAssign};
use limine_protocol_for_rust::requests::memory_map::{MemoryRegionInfo, MemoryRegionType};
use limine_protocol_for_rust::util::PointerSlice;
use x86_64::{PhysAddr, VirtAddr};

///Only supports 4 KiB pages
pub trait FrameAllocator {
    ///Allocates one page frame
    ///returns the base of the physical frame
    fn alloc(&mut self) -> Result<PhysAddr, MemoryError>;

    ///Allocates the specified amount of page frames as a contiguous block of memory
    ///returns the base of the lowest allocated physical frame of the contiguous chunk
    fn alloc_contiguous(&mut self, pages: usize) -> Result<PhysAddr, MemoryError>;

    ///Frees the frame with *phys_addr* as its base
    fn free(&mut self, phys_addr: PhysAddr) -> Result<(), MemoryError>;

    ///Same as *free(...)* but deallocates multiple contiguous page frames
    fn free_contiguous(&mut self, phys_addr: PhysAddr, pages: usize) -> Result<(), MemoryError>;
}

///downward growing linear allocator frame
///used for intializing certain parts of memory before better allocators can be used
pub struct LinearFrameAllocator(pub PhysAddr);

impl FrameAllocator for LinearFrameAllocator {
    fn alloc(&mut self) -> Result<PhysAddr, MemoryError> {
        let ret = Ok(self.0.clone());
        self.0.add_assign(PAGE_SIZE as u64);
        ret
    }

    fn alloc_contiguous(&mut self, pages: usize) -> Result<PhysAddr, MemoryError> {
        let ret = Ok(self.0.clone());
        self.0.add_assign((pages * PAGE_SIZE) as u64);
        ret
    }

    fn free(&mut self, _phys_addr: PhysAddr) -> Result<(), MemoryError> {
        panic!("Linear Frame Allocator only supports static allocations")
    }

    fn free_contiguous(&mut self, _phys_addr: PhysAddr, _pages: usize) -> Result<(), MemoryError> {
        panic!("Linear Frame Allocator only supports static allocations")
    }
}

pub(crate) struct BitmapFrameAllocator(pub(crate) BitmapAllocator);

impl BitmapFrameAllocator {
    pub fn new(
        bitmap_size: usize,
        frame_allocator: &mut LinearFrameAllocator,
        page_table: &mut RecursivePageTable,
        mmap_entries: &PointerSlice<MemoryRegionInfo>,
    ) -> Result<BitmapFrameAllocator, MemoryError> {
        let mut allocator = BitmapAllocator::new(bitmap_size, true, frame_allocator, page_table)?;

        //clear all usable memory for use
        mmap_entries
            .iter()
            .filter(|r| r.get_type() == MemoryRegionType::Usable)
            .map(|r| r.base..(r.base + r.length))
            .for_each(|range| {
                allocator
                    .flag_range(
                        &(range.start / PAGE_SIZE as u64..range.end / PAGE_SIZE as u64),
                        false,
                    )
                    .unwrap()
            });

        Ok(BitmapFrameAllocator(allocator))
    }
}

impl FrameAllocator for BitmapFrameAllocator {
    fn alloc(&mut self) -> Result<PhysAddr, MemoryError> {
        self.0.alloc(1).map(|frame| PhysAddr::new(frame))
    }

    fn alloc_contiguous(&mut self, pages: usize) -> Result<PhysAddr, MemoryError> {
        self.0.alloc(pages as u64).map(|frame| PhysAddr::new(frame))
    }

    fn free(&mut self, phys_addr: PhysAddr) -> Result<(), MemoryError> {
        let offset = phys_addr.as_u64() / PAGE_SIZE as u64;
        self.0.free(offset)
    }

    fn free_contiguous(&mut self, phys_addr: PhysAddr, pages: usize) -> Result<(), MemoryError> {
        let offset = phys_addr.as_u64() / PAGE_SIZE as u64;
        self.0.free_range(&(offset..(offset + pages as u64)))
    }
}
