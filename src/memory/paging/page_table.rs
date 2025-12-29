use crate::limine_requests::HHDM_REQUEST;
use crate::memory::{MemoryError, PAGE_SIZE};
use crate::memory::paging::frame_allocation::FrameAllocator;
use crate::{kprintln, pub_pseudo_const, serial_println};
use core::ops::{Add, Index, IndexMut};
use core::ptr;
use limine_protocol_for_rust::requests::LimineRequest;
use x86_64::instructions::{hlt, tlb};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{PageTable, PageTableFlags, PhysFrame};
use x86_64::{PhysAddr, VirtAddr};
use crate::memory::MemoryError::TODOError;

pub_pseudo_const!(flags_r: PageTableFlags = PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE);
pub_pseudo_const!(flags_rw: PageTableFlags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE);
pub_pseudo_const!(flags_rx: PageTableFlags = PageTableFlags::PRESENT);
pub_pseudo_const!(flags_rwx: PageTableFlags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT);


pub(super) const LEVEL4: u64 = 0xFFFF_FFFF_FFFF_F000;
///A simple recursive page table, I opted not the use the one from x86_64 since it can't be modified while it is not active.
///Uses offset mapping for interal page tables so they can be found easily from their physical address.
pub struct RecursivePageTable {
    pub(super) lvl4: *mut PageTable,
    pub internal_offset: u64,
}
impl RecursivePageTable {
    pub fn new(frame_allocator: &mut dyn FrameAllocator) -> Self {
        let internal_offset = HHDM_REQUEST.get_response().unwrap().offset;

        let frame = frame_allocator.alloc().unwrap();

        //the virtual address, where to place the new page table *at*, can't use *phys_page_table_as_virt(...)* since *Self* isn't created yet.
        let at = VirtAddr::new(frame.as_u64()).add(internal_offset);

        let lvl4 = unsafe { new_page_table_level(at.as_mut_ptr()) };

        //Add recursion entry to level 4 page table
        lvl4.index_mut(511)
            .set_frame(PhysFrame::containing_address(frame), flags_rw());

        Self {
            lvl4,
            internal_offset,
        }
    }

    ///maps *from* to *to* with *flags*, very simple
    ///*frame_allocator* is for allocating new frames for possible new page tables
    pub fn map(
        &mut self,
        from: PhysAddr,
        to: VirtAddr,
        flags: PageTableFlags,
        frame_allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MemoryError>{
        //Try to get the lowest level page table that exists for this virtual address, tries the lowest levels first
        for level in 1..=4 {
            if let Some(page_table) = self.get_page_table(level, to) {
                let mut current_table = page_table;
                //Create remaining levels
                for level in (2..=level).rev() {
                    let entry = current_table.index_mut(get_page_index(level, to.as_u64()));
                    if entry.is_unused() {
                        entry.set_addr(frame_allocator.alloc()?, flags_rw())
                    }
                    let next_table: *mut PageTable =
                        self.phys_page_table_as_virt(entry.addr()).as_mut_ptr();

                    current_table = unsafe { &mut *next_table };
                }

                //Throw error if this address has been mapped already
                if !current_table.index(get_page_index(1, to.as_u64())).is_unused() {
                   return Err(TODOError);
                }


                //Finally set the physical address on the lvl1 table
                let index = current_table
                    .index_mut(get_page_index(1, to.as_u64()));
                    index.set_addr(from, flags);
                let t = self.translate(to);
                return Ok(());
            }
        }
        unreachable!()
    }

    pub fn update_flags(&mut self, to: VirtAddr, flags: PageTableFlags) -> Result<(), MemoryError> {
        if let Some(page_table) = self.get_page_table(1, to) {
            if !page_table.index(get_page_index(1, to.as_u64())).is_unused() {
                return Err(TODOError);
            }

            page_table
                .index_mut(get_page_index(1, to.as_u64()))
                .set_flags(flags);
        }
        Ok(())
    }

    ///The same as *map(...)* but can map multiple contiguous page frames.
    pub fn map_contiguous(
        &mut self,
        pages: usize,
        from: PhysAddr,
        to: VirtAddr,
        flags: PageTableFlags,
        frame_allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MemoryError> {
        for page in (0..pages * PAGE_SIZE).step_by(PAGE_SIZE) {
            self.map(
                from.add(page as u64),
                to.add(page as u64),
                flags,
                frame_allocator,
            )?;
        }
        Ok(())
    }

    ///returns the page table of the asked *level* following *to*.
    ///*to* can be truncated up until the asked level and still give the correct page table.  
    fn get_page_table(&mut self, target_lvl: u64, to: VirtAddr) -> Option<&'static mut PageTable> {
        //check if the asked level is valid
        assert!((1..=4).contains(&target_lvl));

        let mut current_table = unsafe { &mut *(self.lvl4) };

        for level in ((target_lvl+1)..=4).rev() {
            if level == 1 {
                return Some(current_table);
            }
            let entry = current_table.index_mut(get_page_index(level, to.as_u64()));
            if entry.is_unused() {
                return None;
            }
            let next_table: *mut PageTable =
                self.phys_page_table_as_virt(entry.addr()).as_mut_ptr();
            current_table = unsafe { &mut *next_table };
        }
        Some(current_table)
    }

    ///translates the virtual address if it has a corresponding physical address.  
    ///pretty self-explanatory
    pub fn translate(&mut self, from: VirtAddr) -> Option<PhysAddr> {
        if let Some(page_table) = self.get_page_table(1, from) {
            let entry = page_table.index(get_page_index(1, from.as_u64()));
            if entry.is_unused() {
                kprintln!("table: {:x}", page_table as *const _ as usize);
                return None;
            }
            return Some(entry.addr().add(from.as_u64() % PAGE_SIZE as u64));
        }
        None
    }

    ///This can **ONLY** be used on page tables since those are the only dynamically allocatable structure that are offset mapped.
    #[inline(always)]
    fn phys_page_table_as_virt(&self, page_table: PhysAddr) -> VirtAddr {
        VirtAddr::new(page_table.as_u64() + self.internal_offset)
    }

    ///Unmaps the given *address*.  
    ///*frame_allocator* is used to free the frames for empty page tables.
    pub fn unmap(
        &mut self,
        pages: usize,
        address: VirtAddr,
        frame_allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MemoryError> {
        todo!()
    }

    #[inline(always)]
    pub fn load(&mut self) {
        //can't use translate because the lvl4 pointer is not valid rn
        let page_table = PhysAddr::new(self.lvl4 as u64 - self.internal_offset);
        let (_, flags) = Cr3::read();
        unsafe {
            Cr3::write(PhysFrame::from_start_address(page_table).unwrap(), flags);
        }

        hlt();

        self.lvl4 = LEVEL4 as *mut PageTable;
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        tlb::flush(VirtAddr::from_ptr(self.lvl4));
    }
}

fn get_page_index(level: u64, addr: u64) -> usize {
    assert!((1..=4).contains(&level), "page level {} does not exist", level);
    ((addr >> (9 * level + 3)) & 0o777) as usize
}

unsafe fn new_page_table_level(at: *mut PageTable) -> &'static mut PageTable {
    ptr::write(at, PageTable::new());
    let pt_ref = at.as_mut().unwrap();
    pt_ref.zero();
    pt_ref
}


fn dbg_page_table(at: &PageTable) {
    serial_println!("Debugging page table...");
    for i in 0..512 {
        if !at.index(i).is_unused() {
            serial_println!("{} => {:x?}", i, at.index(i));
        }
    }

}