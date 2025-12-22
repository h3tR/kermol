use crate::memory::allocated_memory::AllocatedMemory;
use crate::memory::paging::{LinearFrameAllocator, FRAME_ALLOCATOR, PAGE_SIZE, PAGE_TABLE, VIRTUAL_PAGE_ALLOCATOR};
use crate::memory::{AddressPair, MemoryError};
use crate::util::MEBIBYTE;
use crate::{return_if_none, serial_println};
use core::ops::Add;
use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, PhysFrame};
use x86_64::PhysAddr;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const KERNEL_HEAP_SIZE: usize = 4 * MEBIBYTE;
const HEAP_FRAME_COUNT: usize = KERNEL_HEAP_SIZE / PAGE_SIZE;

pub(super) fn init_heap(init_allocator: &mut LinearFrameAllocator) -> Result<(), MemoryError> {
    //manually allocate and page frames as to not use any Heap dependent type
    let heap = {
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        let mut allocator = FRAME_ALLOCATOR.lock();

        let phys_addr = PhysAddr::new(
            return_if_none!(allocator.get_mut(), MemoryError::LockedAllocator)
                .0
                .alloc(HEAP_FRAME_COUNT as u64)?,
        );

        let mut frames: [PhysFrame; KERNEL_HEAP_SIZE / PAGE_SIZE] =
            [PhysFrame::containing_address(PhysAddr::new(0)); HEAP_FRAME_COUNT];

        for i in 0..HEAP_FRAME_COUNT {
            frames[i] = PhysFrame::containing_address(phys_addr.add(PAGE_SIZE as u64 * i as u64));
        }

        let virt_addr = VIRTUAL_PAGE_ALLOCATOR
            .lock()
            .get_mut()
            .unwrap()
            .alloc(HEAP_FRAME_COUNT as u64)?;

        let mut page_table = PAGE_TABLE.lock();
        let page_table = page_table.get_mut().unwrap();

        for i in 0..HEAP_FRAME_COUNT {
            let page = Page::containing_address(virt_addr.add(PAGE_SIZE as u64 * i as u64 ));
            serial_println!("{:x?}",virt_addr.add(PAGE_SIZE as u64 * i as u64 ));
            unsafe {
                match page_table.map_to(page, frames[i], flags, init_allocator) {
                    Ok(flusher) => flusher.flush(),
                    Err(e) => panic!("{:?}",e),//return Err(MappingError),
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
