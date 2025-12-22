pub mod allocated_memory;
pub mod gdt;
mod heap;
mod paging;

use crate::display::vga_text_emulation::VgaColor;
use crate::display::vga_text_writer::kwriter_set_color;
use crate::limine_requests::{HHDM_REQUEST, MEMORY_MAP_REQUEST};
use crate::memory::heap::init_heap;
use crate::memory::paging::frame_allocation::BitmapFrameAllocator;
use crate::memory::paging::{LinearFrameAllocator, VirtualPageAllocator, FRAME_ALLOCATOR, PAGE_SIZE, PAGE_TABLE, VIRTUAL_PAGE_ALLOCATOR};
use crate::{kprint, kprintln};
use core::fmt::Debug;
use limine_protocol_for_rust::requests::memory_map::{MemoryMapResponse, MemoryRegionType};
use limine_protocol_for_rust::requests::LimineRequest;
use x86_64::structures::paging::{OffsetPageTable, PageTable};
use x86_64::{PhysAddr, VirtAddr};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

#[derive(Clone, Copy, Debug)]
pub enum MemoryError {
    OutOfBounds,
    PageNotPresent,
    WriteToReadOnly,
    EmptyAllocation,
    LockedAllocator,
    NoFreeFrame,
    MappingError,
    AlignmentError,
    //Specifically for frame and virt addr allocation, not heap allocation
    AllocationError,
    DoubleFree,
}
///Initializes Page allocation and the kernel heap
pub fn init_memory() -> Result<(), MemoryError> {
    let memory_map_resp = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Memory map request had no response");
    let hhdm_resp = HHDM_REQUEST
        .get_response()
        .expect("HHDM request had no response");

    log_ram_map(memory_map_resp);

    let physical_memory_offset = VirtAddr::new(hhdm_resp.offset);

    PAGE_TABLE.lock().call_once(|| unsafe {
        OffsetPageTable::new(
            get_active_level_4_table(physical_memory_offset),
            physical_memory_offset,
        )
    });
    kprintln!("Page Table Created");

    let mut init_allocator = LinearFrameAllocator::new(memory_map_resp);
    kprintln!("Temp Linear Frame Allocator Created");

    let vpa = VirtualPageAllocator::new(memory_map_resp, &mut init_allocator)?;
    VIRTUAL_PAGE_ALLOCATOR.lock().call_once(|| vpa);

    let bitmap_frame_allocator = BitmapFrameAllocator::new(memory_map_resp, &mut init_allocator)?;
    FRAME_ALLOCATOR.lock().call_once(|| bitmap_frame_allocator);
    kprintln!("Frame Allocator Initialized");

    init_heap(&mut init_allocator).expect("Heap Initialization failed");
    kprintln!("Kernel Heap created");

    FRAME_ALLOCATOR
        .lock()
        .get_mut()
        .unwrap()
        .mark_init_allocator(init_allocator);

    kprintln!("Frame Allocator Initialized, Linear Frame Allocator obsolete");

    Ok(())
}

fn get_active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let level_4_table_frame = Cr3::read().0;

    let phys_addr = level_4_table_frame.start_address();
    let virt_addr = physical_memory_offset + phys_addr.as_u64();
    let page_table_ptr: *mut PageTable = virt_addr.as_mut_ptr();

    unsafe { &mut *page_table_ptr } // unsafe
}

fn get_frame_count(size: usize) -> usize {
    if size % PAGE_SIZE != 0 {
        1 + size / PAGE_SIZE
    } else {
        size / PAGE_SIZE
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AddressPair(pub VirtAddr, pub PhysAddr);



///Outputs the ram map to the kernel log
fn log_ram_map(memory_map_resp: &'static MemoryMapResponse) {
    kprintln!("Bootloader provided phys RAM map:");
    memory_map_resp.get_entries().iter().for_each(|r| {
        kprint!("[0x{:0>12X}->0x{:0>12X}]: ", r.base, r.base + r.length);
        let region_type = r.get_type();
        kwriter_set_color(match region_type {
            MemoryRegionType::Usable => VgaColor::LightGreen,
            MemoryRegionType::Reserved | MemoryRegionType::BadMemory => VgaColor::LightRed,
            MemoryRegionType::Framebuffer => VgaColor::LightBlue,
            _ => VgaColor::DarkGray,
        } as u32);
        kprintln!("{:?}", region_type);
        kwriter_set_color(VgaColor::LightGray as u32);
    });
}

#[macro_export]
macro_rules! page {
    ($size:expr, $flags:expr) => {
        AllocatedMemory::new($size, $flags | PageTableFlags::PRESENT)
    };

    ($size:expr) => {
        AllocatedMemory::new($size, PageTableFlags::PRESENT)
    };
}
