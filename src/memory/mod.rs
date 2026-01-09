pub mod allocated_memory;
pub mod gdt;
mod heap;
mod paging;

use crate::display::vga_text_emulation::VgaColor;
use crate::display::vga_text_writer::kwriter_set_color;
use crate::limine_requests::{HHDM_REQUEST, MEMORY_MAP_REQUEST};
use crate::memory::heap::{KERNEL_HEAP_SIZE, init_heap};
use crate::memory::paging::frame_allocation::LinearFrameAllocator;
use crate::memory::paging::page_table::flags_rw;
use crate::memory::paging::{FrameAllocator, KernelPagingController, PagingError, init_paging};
use crate::util::KIBIBYTE;
use crate::{kprint, kprintln, serial_println};
use core::fmt::Debug;
use core::ops::{Add, Sub};
use limine_protocol_for_rust::requests::LimineRequest;
use limine_protocol_for_rust::requests::memory_map::{MemoryRegionInfo, MemoryRegionType};
use limine_protocol_for_rust::util::PointerSlice;
use spin::{Mutex, Once};
use x86_64::{PhysAddr, VirtAddr};

pub const PAGE_SIZE: usize = 4 * KIBIBYTE;

pub static KERNEL_PAGING_CONTROLLER: Mutex<Once<KernelPagingController>> = Mutex::new(Once::new());

#[derive(Clone, Copy, Debug)]
pub enum MemoryError {
    OutOfBounds,
    PageNotPresent,
    WriteToReadOnly,
    EmptyAllocation,
    LockedAllocator,
    NoFreeFrame,
    PagingError(PagingError),
    AlignmentError,
    //Specifically for frame and virt addr allocation, not heap allocation
    AllocationError,
    DoubleFree,
    //TODO: resolve all the crap that depends on this
    TODOError,
}

///Initializes Page allocation and the kernel heap
pub fn init_memory(entry_stack_pointer: u64) -> Result<(), MemoryError> {
    let rammap_entries = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Memory map request had no response")
        .get_entries();
    let hhdm_resp = HHDM_REQUEST
        .get_response()
        .expect("HHDM request had no response");

    log_ram_map(&rammap_entries);

    //retrieves the initial virtual memory offset from the HHDM request for Limine
    let physical_memory_offset = VirtAddr::new(hhdm_resp.offset);

    //Finds the largest memory region, this place will surely fit the allocators and initial page tables
    let allocator_region = rammap_entries
        .iter()
        .filter(|region| region.get_type() == MemoryRegionType::Usable)
        .max_by(|region1, region2| region1.length.cmp(&region2.length));

    //We can't really add a check here if the allocator region is large enough because we don't conclusively know what the total size of the used memory.
    if allocator_region.is_none() {
        return Err(MemoryError::AllocationError);
    }
    let allocator_region = allocator_region.unwrap();

    let allocator_phys_base = PhysAddr::new(allocator_region.base);
    let allocator_virt_base =
        VirtAddr::new(allocator_region.base).add(physical_memory_offset.as_u64());

    let mut dummy_allocator = LinearFrameAllocator(PhysAddr::new(
        allocator_virt_base.sub(physical_memory_offset),
    ));

    let heap_pages = KERNEL_HEAP_SIZE / PAGE_SIZE;

    let heap_phys = dummy_allocator.alloc_contiguous(heap_pages)?;

    let mut k_paging_ctrl = unsafe {
        init_paging(
            VirtAddr::new(entry_stack_pointer),
            &mut dummy_allocator,
            &rammap_entries,
        )?
    };

    let heap_virt =
        VirtAddr::new(heap_phys.as_u64()).add(k_paging_ctrl.k_page_table.internal_offset);

    //Map the heap
    k_paging_ctrl.k_page_table.map_contiguous(
        heap_pages,
        heap_phys,
        heap_virt,
        flags_rw(),
        &mut dummy_allocator,
    )?;

    //mark the space used by the dummy_allocator in the frame allocator
    k_paging_ctrl.k_frame_allocator.0.flag_range(
        &(allocator_phys_base.as_u64() / PAGE_SIZE as u64
            ..dummy_allocator.0.as_u64() / PAGE_SIZE as u64),
        true,
    )?;

    //Switch the page table to our new one
    k_paging_ctrl.k_page_table.load();

    serial_println!("PEIS");

    //Initialize the heap after being mapped properly.
    unsafe {
        init_heap(heap_virt);
    }

    //Move the kpc into a static variable so it can be accessed anywhere
    KERNEL_PAGING_CONTROLLER.lock().call_once(|| k_paging_ctrl);

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct AddressPair(pub VirtAddr, pub PhysAddr);

///Outputs the ram map to the kernel log
fn log_ram_map(rammap_entries: &PointerSlice<MemoryRegionInfo>) {
    kprintln!("Bootloader provided physical RAM map:");
    rammap_entries.iter().for_each(|r| {
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
