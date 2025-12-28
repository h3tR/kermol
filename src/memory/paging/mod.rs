use alloc::slice;
pub(crate) use crate::memory::PAGE_SIZE;
pub(crate) use crate::memory::paging::page_mapping::VirtualPageAllocator;
use core::ops::{Add, AddAssign, IndexMut};
use core::ptr;
use limine_protocol_for_rust::requests::memory_map::MemoryRegionInfo;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{FrameAllocator, PageTable, PageTableFlags, PhysFrame, Size4KiB};

pub(super) mod bitmap_allocator;
pub(super) mod frame_allocation;
pub(super) mod page_mapping;

///downward growing dummy linear allocator for initialization of the real allocators
pub struct LinearFrameAllocator(pub PhysAddr);

unsafe impl FrameAllocator<Size4KiB> for LinearFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let allocated_frame = PhysFrame::containing_address(self.0);
        self.0.add_assign(PAGE_SIZE as u64);
        Some(allocated_frame)
    }
}


//Creates a new recursive page table and maps the kernel, currently does not support kernel size over 2 MiB: TODO: MAP THE STACK YOU FUCKING DIPSHIT
pub fn new_page_table(address: VirtAddr, phys_addr: PhysAddr, kernel_region: &MemoryRegionInfo, kernel_address: VirtAddr) -> &'static mut [PageTable] {
    let mut page_tables: [PageTable; 4] = [PageTable::new(), PageTable::new(), PageTable::new(), PageTable::new()];
    //Add recursion entry to level 4 page table
    page_tables[0].index_mut(511).set_frame(
        PhysFrame::containing_address(phys_addr),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    for i in 0..3 {
        page_tables[i].index_mut(get_page_index(4 - i, kernel_address.as_u64())).set_frame(
            PhysFrame::containing_address(phys_addr.add((i * PAGE_SIZE) as u64)),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );
    }

    for frame in kernel_region.base / PAGE_SIZE as u64..(kernel_region.base + kernel_region.length) / PAGE_SIZE as u64 {
        page_tables[3].index_mut(get_page_index(1, frame)).set_frame(
            PhysFrame::containing_address(PhysAddr::new(frame)),
            PageTableFlags::PRESENT | PageTableFlags::GLOBAL,
        );
    }

    unsafe {
        ptr::write(address.as_mut_ptr(), page_tables);

        slice::from_raw_parts_mut(address.as_mut_ptr(), 4)
    }
}




fn get_page_index(level: usize, addr: u64) -> usize {
    if !(1..3).contains(&level) {
        panic!("page level {} does not exist", level);
    }

    ((addr >> (9 * level + 3)) & 0b111111111) as usize
}