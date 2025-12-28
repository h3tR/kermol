use crate::memory::paging::bitmap_allocator::BitmapAllocator;
use crate::memory::{MemoryError, PAGE_SIZE};
use crate::{kprintln, serial_println};
use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::ops::Range;
use limine_protocol_for_rust::requests::memory_map::{MemoryRegionInfo, MemoryRegionType};
use limine_protocol_for_rust::util::PointerSlice;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

pub(crate) struct PhysicalFrameAllocator(pub(crate) BitmapAllocator);

impl PhysicalFrameAllocator {
    pub fn new(
        bitmap_address: VirtAddr,
        bitmap_size: usize,
        mmap_entries: &PointerSlice<MemoryRegionInfo>,
    ) -> Result<PhysicalFrameAllocator, MemoryError> {
        let mut allocator = BitmapAllocator::new(bitmap_address.as_mut_ptr(), bitmap_size, true)?;

        //clear all usable memory for use
        mmap_entries
            .iter()
            .filter(|r| r.get_type() == MemoryRegionType::Usable)
            .map(|r| r.base..(r.base + r.length))
            .for_each(|range| {
                allocator.flag_range(
                    range.start / PAGE_SIZE as u64..range.end / PAGE_SIZE as u64,
                    false,
                )
            });

        serial_println!("Created Frame Allocator at {:?}", bitmap_address);

        Ok(PhysicalFrameAllocator(allocator))
    }

    pub fn alloc(&mut self, frames: u64) -> Result<Vec<PhysFrame>, MemoryError> {
        self.0.alloc(frames).map(|frame| {
            let start_addr = frame * PAGE_SIZE as u64;
            let mut frame_vec = Vec::with_capacity(frames as usize);

            //kprintln!("Allocated frames {:X}-{:X}", frame,frame + frames);

            for frame in 0..frames {
                frame_vec.push(PhysFrame::containing_address(PhysAddr::new(
                    start_addr + frame * PAGE_SIZE as u64,
                )));
            }

            frame_vec
        })
    }

    pub fn alloc_at(
        &mut self,
        frame: PhysFrame,
        frames: u64,
    ) -> Result<Vec<PhysFrame>, MemoryError> {
        let offset = frame.start_address().as_u64() / PAGE_SIZE as u64;

        self.0.alloc_at(offset, frames).map(|frame| {
            let start_addr = frame * PAGE_SIZE as u64;
            let mut frame_vec = Vec::new();
            for frame in 0..frames {
                frame_vec.push(PhysFrame::containing_address(PhysAddr::new(
                    start_addr + frame * PAGE_SIZE as u64,
                )));
            }
            frame_vec
        })
    }

    pub fn free(&mut self, frame: PhysFrame) -> Result<(), MemoryError> {
        let offset = frame.start_address().as_u64() / PAGE_SIZE as u64;
        self.0.free(offset)
    }
}

unsafe impl FrameAllocator<Size4KiB> for PhysicalFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        match self.alloc(1).map(|frames| {
            frames
                .first()
                .expect("A problem occurred allocating")
                .to_owned()
        }) {
            Ok(frame) => Some(frame),
            Err(_) => None,
        }
    }
}

impl FrameDeallocator<Size4KiB> for PhysicalFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.free(frame).expect("frame deallocation failed");
    }
}
