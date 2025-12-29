use crate::limine_requests::KERNEL_ADDRESS_REQUEST;
pub(crate) use crate::memory::paging::frame_allocation::{
    BitmapFrameAllocator, FrameAllocator, LinearFrameAllocator,
};
pub(crate) use crate::memory::paging::page_mapping::VirtualMemoryAllocator;
use crate::memory::paging::page_table::{flags_r, flags_rw, flags_rwx, RecursivePageTable};
pub(crate) use crate::memory::paging::paging_controller::KernelPagingController;
pub(crate) use crate::memory::PAGE_SIZE;
use core::ops::{Add, IndexMut, Sub};
use limine_protocol_for_rust::requests::memory_map::{MemoryRegionInfo, MemoryRegionType};
use limine_protocol_for_rust::requests::LimineRequest;
use limine_protocol_for_rust::util::PointerSlice;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};
use crate::kprintln;

pub(super) mod bitmap_allocator;
pub(super) mod frame_allocation;
pub(super) mod page_mapping;
pub(super) mod page_table;
pub mod paging_controller;

unsafe extern "C" {
    static _text_start: u8;
    static _rodata_start: u8;
    static _data_start: u8;

    //No need for '_bss_start' since data and bss need the same page table flags
    static _elf_end: u8;

}

#[inline(always)]
pub fn init_paging(
    linear_frame_allocator: &mut LinearFrameAllocator,
    rammap_entries: &PointerSlice<MemoryRegionInfo>,
) -> KernelPagingController {
    let mut k_page_table = RecursivePageTable::new(linear_frame_allocator);

    //find the kernel executable in memory and map it
    let kernel_region = rammap_entries
        .iter()
        .find(|region| region.get_type() == MemoryRegionType::ExecutableAndModules)
        .expect("Memory map had no executable");

    //map the kernel
    map_kernel(kernel_region, &mut k_page_table, linear_frame_allocator);

    //map important regions that might/will be used later.
    map_misc(rammap_entries, &mut k_page_table, linear_frame_allocator);

    //finds the valid memory region at the highest address
    //excludes reserved and framebuffer region types, because they can fall outside the phys memory range.
    let highest_valid_region = rammap_entries
        .iter()
        .filter(|region| {
            region.get_type() != MemoryRegionType::Reserved
                && region.get_type() != MemoryRegionType::Framebuffer
        })
        .max_by(|r, a| (r.base + r.length).cmp(&(a.base + a.length)))
        .expect("No valid memory region");

    //Calculate the size of the frame allocator/half the size of the virtual memory allocator in memory;
    let size_bytes =
        (highest_valid_region.base + highest_valid_region.length) / PAGE_SIZE as u64 / 8;

    let k_frame_allocator = BitmapFrameAllocator::new(
        size_bytes as usize,
        linear_frame_allocator,
        &mut k_page_table,
        &rammap_entries,
    )
    .unwrap();

    //Create a virtual memory allocator with twice the capacity of the physical memory size
    let k_virt_mem_allocator = VirtualMemoryAllocator::new(
        size_bytes as usize * 2,
        linear_frame_allocator,
        &mut k_page_table,
    )
    .unwrap();

    KernelPagingController {
        k_page_table,
        k_frame_allocator,
        k_virt_mem_allocator,
    }
}

///maps the kernel executable
#[inline(always)]
fn map_kernel(
    kernel_region: &MemoryRegionInfo,
    page_table: &mut RecursivePageTable,
    allocator: &mut LinearFrameAllocator,
) {
    let kernel_addr = KERNEL_ADDRESS_REQUEST.get_response().unwrap();

    let kernel_pages = size_in_pages(kernel_region.length as usize);
    //TODO: distinguish between .text, .data, etc. for them to have appropriate flags

    page_table.map_contiguous(
        kernel_pages,
        PhysAddr::new(kernel_addr.physical_base as u64),
        VirtAddr::new(kernel_addr.virtual_base as u64),
        flags_rwx() | PageTableFlags::GLOBAL,
        allocator,
    ).unwrap();
}

///offset maps important RAM map regions that might/will be used later.
#[inline(always)]
fn map_misc(
    rammap_entries: &PointerSlice<MemoryRegionInfo>,
    page_table: &mut RecursivePageTable,
    allocator: &mut LinearFrameAllocator,
) {
    for region in rammap_entries.iter() {
        let flags = match region.get_type() {
            //TODO: properly map stack seprarately
            MemoryRegionType::Framebuffer | MemoryRegionType::BootloaderReclaimable => flags_rw(),
            MemoryRegionType::AcpiNvs
            | MemoryRegionType::AcpiReclaimable
            | MemoryRegionType::AcpiTables => flags_r(),
            //We don't want to map the other region types so we skip them
            _ => continue,
        };
        let pages = size_in_pages(region.length as usize);

        kprintln!("Mapping {:x}, {:x}", VirtAddr::new(region.base + page_table.internal_offset).as_u64(), region.length);
        page_table.map_contiguous(
            pages,
            PhysAddr::new(region.base),
            VirtAddr::new(region.base + page_table.internal_offset),
            flags,
            allocator,
        ).unwrap();
    }
}

///returns how many pages *size_bytes* would need to fit.
pub fn size_in_pages(size_bytes: usize) -> usize {
    let mut size = size_bytes / PAGE_SIZE;
    if size_bytes % PAGE_SIZE != 0 {
        size += 1;
    }
    size
}

