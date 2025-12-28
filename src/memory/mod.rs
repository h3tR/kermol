pub mod allocated_memory;
pub mod gdt;
mod heap;
mod paging;

use crate::display::vga_text_emulation::VgaColor;
use crate::display::vga_text_writer::kwriter_set_color;
use crate::limine_requests::{HHDM_REQUEST, KERNEL_ADDRESS_REQUEST, MEMORY_MAP_REQUEST};
use crate::memory::heap::{KERNEL_HEAP_SIZE, init_heap};
use crate::memory::paging::frame_allocation::PhysicalFrameAllocator;
use crate::memory::paging::{LinearFrameAllocator, VirtualPageAllocator, new_page_table};
use crate::util::KIBIBYTE;
use crate::{kprint, kprintln, serial_println};
use core::fmt::Debug;
use core::ops::{Add, IndexMut, Sub};
use core::{ptr, slice};
use core::arch::asm;
use limine_protocol_for_rust::requests::LimineRequest;
use limine_protocol_for_rust::requests::memory_map::{MemoryRegionInfo, MemoryRegionType};
use limine_protocol_for_rust::util::PointerSlice;
use spin::{Mutex, Once};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    Mapper, Page, PageTable, PageTableFlags, PhysFrame, RecursivePageTable, Size4KiB, Translate,
};
use x86_64::{PhysAddr, VirtAddr};
use x86_64::instructions::hlt;
use x86_64::structures::paging::mapper::MapToError;
use crate::memory::paging::page_mapping::translate_to_phys;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub const PAGE_SIZE: usize = 4 * KIBIBYTE;

pub static PHYSICAL_FRAME_ALLOCATOR: Mutex<Once<PhysicalFrameAllocator>> = Mutex::new(Once::new());

pub(super) static KERNEL_VIRTUAL_PAGE_ALLOCATOR: Mutex<Once<VirtualPageAllocator>> =
    Mutex::new(Once::new());

pub(super) static KERNEL_PAGE_TABLE: Mutex<Once<RecursivePageTable<'static>>> =
    Mutex::new(Once::new());

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
pub fn init_memory(entry_stack_pointer: u64) -> Result<(), MemoryError> {
    let memmap_entries = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Memory map request had no response")
        .get_entries();
    let hhdm_resp = HHDM_REQUEST
        .get_response()
        .expect("HHDM request had no response");
    let kernel_addr_resp = KERNEL_ADDRESS_REQUEST
        .get_response()
        .expect("kernel address request had no response");

    log_ram_map(&memmap_entries);

    //retrieves the inital virtual memory offset from the HHDM request for Limine
    let physical_memory_offset = VirtAddr::new(hhdm_resp.offset);

    //finds the valid memory region at the highest address
    let highest_valid_region = memmap_entries
        .iter()
        .filter(|region| {
            region.get_type() != MemoryRegionType::Reserved
                && region.get_type() != MemoryRegionType::Framebuffer
        })
        .max_by(|r, a| (r.base + r.length).cmp(&(a.base + a.length)))
        .expect("No valid memory region");

    //Calculate the size of the frame allocator in memory;
    //excludes reserved and framebuffer region types, because they can fall outside the phys memory range.
    //pfa stands for physical frame allocator
    let pfa_byte_size =
        (highest_valid_region.base + highest_valid_region.length) / PAGE_SIZE as u64 / 8;

    //Not sure how large this should be exactly, but I figured twice the physical frames should be enough virtual memory for the kernel.
    //k_vpa stands for kernel virtual page allocator
    let k_vpa_byte_size = pfa_byte_size * 2;


    //Finds the largest memory region, this place will surely fit out allocators and initial page tables
    let allocator_region = memmap_entries
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

    kprintln!("Allocator base: {:x?}", allocator_virt_base);

    let k_page_table_addr = allocator_virt_base;

    let kernel_region = memmap_entries
        .iter()
        .find(|region| region.get_type() == MemoryRegionType::ExecutableAndModules)
        .expect("Memory map had no executable");

    let k_page_tables = new_page_table(
        k_page_table_addr,
        kernel_region,
        VirtAddr::new(entry_stack_pointer + 16),
        physical_memory_offset.as_u64(),
    );

    kprintln!("New page table created. Mapped boot stack and kernel");
    let k_page_table_size = (k_page_tables.len() * size_of::<PageTable>()) as u64;

    let heap_addr = k_page_table_addr.add(k_page_table_size);
    let pfa_addr = heap_addr.add(KERNEL_HEAP_SIZE as u64);
    let k_vpa_addr = pfa_addr.add(pfa_byte_size);

    //combined size of the structures required to set up memory mgmt (allocators + new page table lvl 4)
    let init_memory_size = pfa_byte_size + k_vpa_byte_size + KERNEL_HEAP_SIZE as u64 + k_page_table_size;

    //start the dummy allocator after the allocators
    let dummy_allocator_base = k_vpa_addr.add(k_vpa_byte_size);

    //the dummy allocator is only used to create new page table entries
    let mut dummy_allocator = LinearFrameAllocator(PhysAddr::new(
        dummy_allocator_base.sub(physical_memory_offset),
    ));

    //IMPORTANT: every address before this point that is not pointing to something on the stack
    //or in the kernel executable (consts, statics, etc.) will become invalid!


    //switch to k_page_table instead of using the one provided by limine
    let (_, flags) = Cr3::read();
    unsafe {
        Cr3::write(
            PhysFrame::from_start_address(PhysAddr::new(
                k_page_tables.as_ptr() as u64 - physical_memory_offset.as_u64(),
            ))
            .expect("Tried to set unaligned active page table"),
            flags,
        );
    }


    let new_k_page_table_addr = 0xFFFF_FFFF_FFFF_F000 as *mut PageTable;

    let mut k_page_table = unsafe { RecursivePageTable::new(&mut *new_k_page_table_addr ).unwrap() };

    //TODO: fix addresses used below

    let mut frame_allocator =
        PhysicalFrameAllocator::new(pfa_addr, pfa_byte_size as usize, &memmap_entries)?;

    let mut k_vpa = VirtualPageAllocator::new(k_vpa_addr, k_vpa_byte_size as usize)?;

    //TODO: fix pretty much everything down here

    //map allocators, these are mapped at their physical addr + physical offset, they shouldn't fall into the range for allocating virtual memory
    for i in 1..get_size_in_pages(init_memory_size as usize) as u64 {
        let page: Page<Size4KiB> =
            Page::containing_address(allocator_virt_base.add(i * PAGE_SIZE as u64));

        let frame = allocator_virt_base.add(i * PAGE_SIZE as u64).as_u64();
        unsafe {
            k_page_table
                .map_to(
                    page,
                    PhysFrame::from_start_address(PhysAddr::new(
                        frame - physical_memory_offset.as_u64(),
                    )).unwrap(),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    &mut dummy_allocator,
                )
                .unwrap()
                .flush();
        }
    }

    //map important regions to k_page_table, these regions
    for region in memmap_entries.iter() {
        let region_type = region.get_type();
        for i in 0..region.length / PAGE_SIZE as u64 {
            match region_type {
                //all other types of memory should be read-only and not executable.
                MemoryRegionType::AcpiNvs
                | MemoryRegionType::AcpiReclaimable
                | MemoryRegionType::BootloaderReclaimable
                | MemoryRegionType::AcpiTables => unsafe {
                    let page: Page<Size4KiB> = Page::containing_address(
                        VirtAddr::new(region.base)
                            .add(i * PAGE_SIZE as u64)
                            .add(physical_memory_offset.as_u64()),
                    );
                    let mapping_result = k_page_table
                        .map_to(
                            page,
                            PhysFrame::from_start_address(
                                PhysAddr::new(region.base).add(i * PAGE_SIZE as u64),
                            )
                                .unwrap(),
                            PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE,
                            &mut dummy_allocator,
                        );
                    match mapping_result {
                        Ok(flush) => flush.flush(),
                        Err(e) => match e {
                            //Ignore the already mapped pages because the only mapped region from here is the boot stack
                            MapToError::PageAlreadyMapped(_) => (),
                            _ => panic!("{:?}", e)
                        },
                    }
                },
                _ => (),
            }
        }
    }


    let dummy_allocated_space = (dummy_allocator.0.as_u64() + physical_memory_offset.as_u64())
        - dummy_allocator_base.as_u64();

    //Set all physical frames used for the allocators and dummy allocated space. Ranges have to be divided by the page size since we have raw access to the bitmap allocator.
    frame_allocator.0.flag_range(
        (allocator_virt_base.as_u64() - physical_memory_offset.as_u64()) / PAGE_SIZE as u64
            ..(allocator_virt_base.as_u64() - physical_memory_offset.as_u64()) / PAGE_SIZE as u64
            + get_size_in_pages((init_memory_size + dummy_allocated_space) as usize) as u64,
        true,
    );

    //Initialize the heap after being mapped properly.
    init_heap(heap_addr);

    //Move pfa, k_vpa and k_page_table into static variables so they are usable anywhere.
    PHYSICAL_FRAME_ALLOCATOR
        .lock()
        .call_once(|| frame_allocator);
    KERNEL_VIRTUAL_PAGE_ALLOCATOR.lock().call_once(|| k_vpa);
    KERNEL_PAGE_TABLE.lock().call_once(|| k_page_table);

    Ok(())
}

fn get_size_in_pages(size: usize) -> usize {
    if size % PAGE_SIZE != 0 {
        1 + size / PAGE_SIZE
    } else {
        size / PAGE_SIZE
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AddressPair(pub VirtAddr, pub PhysAddr);

///Outputs the ram map to the kernel log
fn log_ram_map(memmap_entries: &PointerSlice<MemoryRegionInfo>) {
    kprintln!("Bootloader provided physical RAM map:");
    memmap_entries.iter().for_each(|r| {
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
