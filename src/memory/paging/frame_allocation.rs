use crate::memory::paging::bitmap_allocator::BitmapAllocator;
pub(crate) use crate::memory::paging::{LinearFrameAllocator, PAGE_SIZE};
use crate::memory::MemoryError::AlignmentError;
use crate::memory::MemoryError;
use crate::serial_println;
use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use limine_protocol_for_rust::requests::memory_map::{MemoryMapResponse, MemoryRegionType};
use x86_64::structures::paging::{
    FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};
use crate::memory::paging::get_total_frames;

pub(crate) struct BitmapFrameAllocator(pub(crate) BitmapAllocator);

impl BitmapFrameAllocator {
    pub fn new(
        memory_map: &'static MemoryMapResponse,
        init_allocator: &mut LinearFrameAllocator,
    ) -> Result<BitmapFrameAllocator, MemoryError> {
        //calculate total frames in memory
        let total_frames = get_total_frames(memory_map);

        let paging_bitmap_size = (total_frames / 8) as usize;

        let contiguous_frames = memory_map
            .get_entries()
            .iter()
            .map(|r| r.base..(r.base + r.length))
            .flat_map(|r| r.step_by(PAGE_SIZE))
            .filter(|x| x / 4096 < total_frames)
            .count() as u64;

        let bitmap_size = (contiguous_frames / 8) as usize;

        for frame_index in 0..bitmap_size {
            let frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(
                (total_frames / 2 + paging_bitmap_size as u64 + frame_index as u64)
                    * PAGE_SIZE as u64,
            ))
            .map_err(|_| AlignmentError)?;

            init_allocator.identity_map(frame)?;
        }

        let addr = VirtAddr::new((total_frames / 2 + paging_bitmap_size as u64) * PAGE_SIZE as u64);



        let mut allocator = BitmapAllocator::new(addr.as_mut_ptr(), total_frames as usize, total_frames / 2 + paging_bitmap_size as u64 + bitmap_size as u64);

        //kprintln!("Allocated frames {:X}-{:X} for allocation bitmaps", total_frames/2,total_frames/2 + bitmap_size as u64 + paging_bitmap_size as u64-1);

        //mark bitmap frames
        for frame in 0..(bitmap_size + paging_bitmap_size) {
            allocator.set(total_frames / 2 + frame as u64)
        }

        //reserve unusable memory regions from bitmap
        for address in memory_map
            .get_entries()
            .iter()
            .filter(|r| r.get_type() != MemoryRegionType::Usable)
            .map(|r| r.base..(r.base + r.length))
            .flat_map(|r| r.step_by(PAGE_SIZE))
        {
            let frame = address / PAGE_SIZE as u64;
            if frame < contiguous_frames {
                allocator.set(frame);
            }
        }
        serial_println!("Created Frame Allocator at {:?}", addr);

        Ok(BitmapFrameAllocator(allocator))
    }

    pub(crate) fn mark_init_allocator(&mut self, init_allocator: LinearFrameAllocator) {
        //kprintln!("Allocated frames {:X}-{:X} with init allocator", init_allocator.base_frame - init_allocator.allocated_frames,init_allocator.base_frame-1);

        for frame in 0..init_allocator.allocated_frames {
            self.0.set(init_allocator.base_frame - frame)
        }
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

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
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

impl FrameDeallocator<Size4KiB> for BitmapFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.free(frame).expect("frame deallocation failed");
    }
}


