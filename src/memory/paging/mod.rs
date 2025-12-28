use crate::limine_requests::{KERNEL_ADDRESS_REQUEST, STACK_SIZE_REQUEST};
pub(crate) use crate::memory::paging::page_mapping::VirtualPageAllocator;
pub(crate) use crate::memory::PAGE_SIZE;
use crate::{kprintln, serial_println};
use alloc::slice;
use core::hash::Hasher;
use core::ops::{Add, AddAssign, Index, IndexMut};
use core::ptr;
use limine_protocol_for_rust::requests::executable_address::ExecutableAddressResponse;
use limine_protocol_for_rust::requests::memory_map::MemoryRegionInfo;
use limine_protocol_for_rust::requests::LimineRequest;
use x86_64::structures::paging::{FrameAllocator, PageTable, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

pub(super) mod bitmap_allocator;
pub(super) mod frame_allocation;
pub(super) mod page_mapping;

///downward growing dummy linear allocator for initialization of the real allocators
pub struct LinearFrameAllocator(pub PhysAddr);

unsafe impl FrameAllocator<Size4KiB> for LinearFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let allocated_frame = PhysFrame::from_start_address(self.0).unwrap();
        self.0.add_assign(PAGE_SIZE as u64);
        Some(allocated_frame)
    }
}

///Creates a new recursive page table and maps the kernel, and boot stack, index 0 has the lvl 4 table
pub fn new_page_table(
    address: VirtAddr,
    kernel_region: &MemoryRegionInfo,
    stack_top: VirtAddr,
    phys_offset: u64,
) -> &'static mut [PageTable] {
    //create lvl 4 page table
    //new_entries is used as a pointer to the next free space that can be used for new page table levels.
    let mut lvl4 = unsafe { new_page_table_level(address.as_mut_ptr()) };
    let mut new_entries = unsafe { address.as_mut_ptr::<PageTable>().add(1)};
    //Add recursion entry to level 4 page table
    lvl4.index_mut(511).set_frame(
        PhysFrame::containing_address(PhysAddr::new(address.as_u64() - phys_offset)),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    kprintln!("Level 4 page table created");

    new_entries = init_stack_mapping(stack_top, lvl4, new_entries, phys_offset);

    kprintln!("boot stack mapped");


    new_entries = init_kernel_mapping(
        kernel_region,
        KERNEL_ADDRESS_REQUEST
            .get_response()
            .expect("Invalid Limine kernel address response"),
        lvl4,
        new_entries,
        phys_offset,
    );

    kprintln!("Kernel mapped");

    let page_tables = (new_entries as u64 - address.as_u64()) as usize / size_of::<PageTable>();

    unsafe { slice::from_raw_parts_mut(address.as_mut_ptr(), page_tables) }
}

///maps the kernel executable
fn init_kernel_mapping(
    kernel_region: &MemoryRegionInfo,
    kernel_addr: &ExecutableAddressResponse,
    lvl4: &mut PageTable,
    new_entries: *mut PageTable,
    phys_offset: u64,
) -> *mut PageTable {
    let mut new_entries = new_entries;

    for page in (0..kernel_region.length).step_by(PAGE_SIZE) {
        new_entries = map_entry(
            PhysAddr::new(kernel_addr.physical_base as u64).add(page),
            VirtAddr::new(kernel_addr.virtual_base as u64).add(page),
            lvl4,
            new_entries,
            //TODO: differentiate executable linker sections (.text should ideally not be writable)
            PageTableFlags::PRESENT | PageTableFlags::GLOBAL | PageTableFlags::WRITABLE,
            phys_offset,
        );
    }

    new_entries
}

///maps the boot stack provided by limine
fn init_stack_mapping(
    stack_top: VirtAddr,
    lvl4: &mut PageTable,
    new_entries: *mut PageTable,
    phys_offset: u64,
) -> *mut PageTable {
    let mut new_entries = new_entries;


    //Leaves a guard page unmapped on top of the stack

    let virt_stack_bottom = stack_top.as_u64() - STACK_SIZE_REQUEST.stack_size;

    //Map the stack pages
    for page in (0..STACK_SIZE_REQUEST.stack_size).step_by(PAGE_SIZE) {
        new_entries = map_entry(
            as_phys_addr(virt_stack_bottom + page, phys_offset),
            VirtAddr::new(virt_stack_bottom).add(page),
            lvl4,
            new_entries,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
            phys_offset,
        );
    }

    new_entries
}

fn map_entry(
    from: PhysAddr,
    to: VirtAddr,
    lvl4: &mut PageTable,
    new_entries: *mut PageTable,
    flags: PageTableFlags,
    phys_offset: u64,
) -> *mut PageTable {
    let mut new_entries = new_entries;
    let mut current_table = lvl4;
    for level in (1..=4).rev() {
        let index = current_table.index_mut(get_page_index(level, to.as_u64()));
        if index.is_unused() {
            unsafe { new_page_table_level(new_entries) };
            match level {
                1 => index.set_addr(from, flags),
                _ => index.set_addr(
                    as_phys_addr(new_entries as u64, phys_offset),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                ),
            }
             new_entries = unsafe {new_entries.add(1)};
        }
        current_table = unsafe { &mut *((index.addr().as_u64() + phys_offset) as *mut PageTable) };
    }
    new_entries
}

fn translate(
    page: VirtAddr,
    lvl4: &mut PageTable,
    phys_offset: u64
) -> PhysAddr {
    let mut current_table = lvl4;
    for level in (1..=4).rev() {
        let index = current_table.index_mut(get_page_index(level, page.as_u64()));
        if index.is_unused() {
            panic!("UNUSED");
        }
        if level == 1 {
            return index.addr();
        }
        let next_table = (index.addr().as_u64() + phys_offset) as *mut PageTable;
        current_table = unsafe { &mut *next_table };
    }
    unreachable!();
}

unsafe fn new_page_table_level(at: *mut PageTable) -> &'static mut PageTable {
    ptr::write(at, PageTable::new());
    let pt_ref = at.as_mut().unwrap();
    pt_ref.zero();
    pt_ref
}

fn get_page_index(level: usize, addr: u64) -> usize {
    if !(1..=4).contains(&level) {
        panic!("page level {} does not exist", level);
    }

    ((addr >> (9 * level + 3)) & 0o777) as usize
}

fn as_phys_addr(addr: u64, phys_offset: u64) -> PhysAddr {
    PhysAddr::new(addr - phys_offset)
}

