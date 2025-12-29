use crate::limine_requests::STACK_SIZE_REQUEST;
pub(crate) use crate::memory::paging::frame_allocation::{
    BitmapFrameAllocator, FrameAllocator, LinearFrameAllocator,
};
pub(crate) use crate::memory::paging::page_mapping::VirtualMemoryAllocator;
use crate::memory::paging::page_table::{flags_r, flags_rw, flags_rx, RecursivePageTable};
pub(crate) use crate::memory::paging::paging_controller::KernelPagingController;
use crate::memory::MemoryError;
pub(crate) use crate::memory::PAGE_SIZE;
use core::ops::{Add, IndexMut, Sub};
use limine_protocol_for_rust::requests::memory_map::{MemoryRegionInfo, MemoryRegionType};
use limine_protocol_for_rust::util::PointerSlice;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};

pub(super) mod bitmap_allocator;
pub(super) mod frame_allocation;
pub(super) mod page_mapping;
pub(super) mod page_table;
pub mod paging_controller;

unsafe extern "C" {
    static _limine_reqs_start: u8;
    static _text_start: u8;
    static _rodata_start: u8;
    static _data_start: u8;
    //No need for '_bss_start' since data and bss need the same page table flags, we see both as one region
    static _elf_end: u8;

}

#[inline(always)]
pub fn init_paging(
    entry_stack_pointer: u64,
    linear_frame_allocator: &mut LinearFrameAllocator,
    rammap_entries: &PointerSlice<MemoryRegionInfo>,
) -> Result<KernelPagingController, MemoryError> {
    let mut k_page_table = RecursivePageTable::new(linear_frame_allocator);

    //map the kernel
    map_kernel(&mut k_page_table, linear_frame_allocator)?;

    //map important regions that might/will be used later.
    map_misc(rammap_entries, &mut k_page_table, linear_frame_allocator)?;
    
    //remap the stack to be writable too.
    remap_stack(entry_stack_pointer, &mut k_page_table, linear_frame_allocator)?;
    
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
    )?;

    //Create a virtual memory allocator with twice the capacity of the physical memory size
    let k_virt_mem_allocator = VirtualMemoryAllocator::new(
        size_bytes as usize * 2,
        linear_frame_allocator,
        &mut k_page_table,
    )?;

    Ok(KernelPagingController {
        k_page_table,
        k_frame_allocator,
        k_virt_mem_allocator,
    })
}

///maps the kernel executable
#[inline(always)]
fn map_kernel(
    page_table: &mut RecursivePageTable,
    allocator: &mut LinearFrameAllocator,
) -> Result<(), MemoryError> {

    map_kernel_section(unsafe {&_limine_reqs_start}, unsafe {&_text_start}, flags_r(), page_table, allocator)?;
    map_kernel_section(unsafe {&_text_start}, unsafe {&_rodata_start}, flags_rx(), page_table, allocator)?;
    map_kernel_section(unsafe {&_rodata_start}, unsafe {&_data_start}, flags_r(), page_table, allocator)?;
    map_kernel_section(unsafe {&_data_start}, unsafe {&_elf_end}, flags_rw(), page_table, allocator)?;

    Ok(())
}

fn map_kernel_section(
    region_start: &u8, 
    region_end: &u8, 
    flags: PageTableFlags,
    page_table: &mut RecursivePageTable,
    allocator: &mut LinearFrameAllocator
) -> Result<(), MemoryError>  {
    let start = region_start as *const _ as usize;
    let end = region_end as *const _ as usize;
    for page in (start..end).step_by(PAGE_SIZE) {
        page_table
            .map(
                PhysAddr::new(page  as u64 - page_table.internal_offset),
                VirtAddr::new(page as u64),
                flags | PageTableFlags::GLOBAL,
                allocator,
            )?;
    }
    Ok(())
}

///offset maps important RAM map regions that might/will be used later.
#[inline(always)]
fn map_misc(
    rammap_entries: &PointerSlice<MemoryRegionInfo>,
    page_table: &mut RecursivePageTable,
    allocator: &mut LinearFrameAllocator,
) -> Result<(), MemoryError> {
    for region in rammap_entries.iter() {
        let flags = match region.get_type() {
            MemoryRegionType::Framebuffer => flags_r(),
            MemoryRegionType::AcpiNvs
            | MemoryRegionType::AcpiReclaimable
            | MemoryRegionType::AcpiTables
            | MemoryRegionType::BootloaderReclaimable => flags_r(),
            //We don't want to map the other region types so we skip them
            _ => continue,
        };
        let pages = size_in_pages(region.length as usize);
        page_table
            .map_contiguous(
                pages,
                PhysAddr::new(region.base),
                VirtAddr::new(region.base + page_table.internal_offset),
                flags,
                allocator,
            )?;
    }
    Ok(())
}

#[inline(always)]
fn remap_stack(
    entry_stack_pointer: u64,
    page_table: &mut RecursivePageTable,
    allocator: &mut LinearFrameAllocator,
) -> Result<(), MemoryError> {
    let actual_stack_pointer = entry_stack_pointer + 16;

    let stack_bottom = actual_stack_pointer - STACK_SIZE_REQUEST.stack_size;

    //TODO unmap guard page
    page_table.unmap(1, VirtAddr::new(actual_stack_pointer), allocator)?;

    for page in (stack_bottom..actual_stack_pointer).step_by(PAGE_SIZE) {
        page_table.update_flags(
            VirtAddr::new(page),
            flags_rw()
        )?;
    }
    Ok(())

}

///returns how many pages *size_bytes* would need to fit.
pub fn size_in_pages(size_bytes: usize) -> usize {
    let mut size = size_bytes / PAGE_SIZE;
    if size_bytes % PAGE_SIZE != 0 {
        size += 1;
    }
    size
}
