use crate::memory::MemoryError::MappingError;
use crate::memory::allocated_memory::AllocatedMemory;
use crate::memory::paging::{LinearFrameAllocator, FRAME_ALLOCATOR, PAGE_SIZE, PAGE_TABLE, VIRTUAL_PAGE_ALLOCATOR};
use crate::memory::{AddressPair, MemoryError};
use crate::util::MEBIBYTE;
use crate::{return_if_none, serial_println};
use core::ops::Add;
use linked_list_allocator::LockedHeap;
use x86_64::PhysAddr;
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, PhysFrame};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const KERNEL_HEAP_SIZE: usize = 4 * MEBIBYTE;

pub(super) fn init_heap(init_allocator: &mut LinearFrameAllocator) -> Result<(), MemoryError> {
    //manually allocate and page frames as to not use any Heap dependent type
    let heap = {
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        let mut allocator = FRAME_ALLOCATOR.lock();

        let frame_count = (KERNEL_HEAP_SIZE / PAGE_SIZE) as u64;

        let mut frames: [PhysFrame; KERNEL_HEAP_SIZE / PAGE_SIZE] =
            [PhysFrame::containing_address(PhysAddr::new(0)); KERNEL_HEAP_SIZE / PAGE_SIZE];
        let phys_addr = PhysAddr::new(
            return_if_none!(allocator.get_mut(), MemoryError::LockedAllocator)
                .0
                .alloc(frame_count)?,
        );

        for i in 0..frame_count {
            frames[i as usize] = PhysFrame::containing_address(phys_addr.add(PAGE_SIZE as u64 * i));
        }

        let virt_addr = VIRTUAL_PAGE_ALLOCATOR
            .lock()
            .get_mut()
            .unwrap()
            .alloc(frame_count)?;

        let mut page_table = PAGE_TABLE.lock();
        let page_table = page_table.get_mut().unwrap();

        for i in 0..frame_count {
            let page = Page::containing_address(virt_addr.add(PAGE_SIZE as u64 * i));

            unsafe {
                match page_table.map_to(page, frames[i as usize], flags, init_allocator) {
                    Ok(flusher) => flusher.flush(),
                    Err(_) => return Err(MappingError),
                }
            }
        }

        serial_println!("Initialized Kernel heap at {:?}", virt_addr);

        AllocatedMemory {
            address: AddressPair(virt_addr, phys_addr),
            size: KERNEL_HEAP_SIZE,
            flags,
            free_after_use: false,
        }
    };

    unsafe {
        ALLOCATOR
            .lock()
            .init(heap.address.0.as_u64() as usize, KERNEL_HEAP_SIZE);
    }

    Ok(())
}
