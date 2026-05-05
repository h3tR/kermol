use crate::memory::paging::{FrameAllocator, KernelPagingController, PAGE_SIZE};
use crate::memory::MemoryError::{
    EmptyAllocation, LockedAllocator, OutOfBounds, PageNotPresent, WriteToReadOnly,
};
use crate::memory::{AddressPair, MemoryError, KERNEL_PAGING_CONTROLLER};
use crate::return_if_none;
use core::ops::Add;
use core::ptr;
use x86_64::structures::paging::PageTableFlags;
use x86_64::PhysAddr;

pub struct AllocatedMemory {
    pub address: AddressPair,
    pub(super) size: usize,
    pub flags: PageTableFlags,
    pub(super) free_after_use: bool,
}
impl AllocatedMemory {
    pub fn new(size: usize, flags: PageTableFlags) -> Result<Self, MemoryError> {
        if size == 0 {
            return Err(EmptyAllocation);
        }
        let pages = size.div_ceil(PAGE_SIZE);
        let mut paging_ctl: KernelPagingController =
            return_if_none!(KERNEL_PAGING_CONTROLLER.lock().get_mut(), LockedAllocator)?;

        let phys_addr = paging_ctl.frame_allocator.alloc_contiguous(pages)?;

        let virt_addr = paging_ctl.virt_mem_allocator.alloc_contiguous(pages)?;

        paging_ctl.rec_page_table.map_contiguous(
            pages,
            phys_addr,
            virt_addr,
            flags,
            &mut paging_ctl.frame_allocator,
        )?;

        Ok(Self {
            address: AddressPair(virt_addr, phys_addr),
            size,
            flags,
            free_after_use: true,
        })
    }
    /// This function is for allocating memory with a set physical location, only virtual memory is allocated and mapped.
    /// Should be used for things like MMIO and other memory structures that cannot be defined and moved by the kernel.
    /// These pages are also not deallocated when they are dropped, altough their virtual memory will be freed.
    pub fn reserve_physical(
        phys_addr: PhysAddr,
        size: usize,
        flags: PageTableFlags,
    ) -> Result<Self, MemoryError> {
        if size == 0 {
            return Err(EmptyAllocation);
        }
        let pages = size.div_ceil(PAGE_SIZE);
        let mut paging_ctl: KernelPagingController =
            return_if_none!(KERNEL_PAGING_CONTROLLER.lock().get_mut(), LockedAllocator)?;

        let virt_addr = paging_ctl.virt_mem_allocator.alloc_contiguous(pages)?;

        paging_ctl.rec_page_table.map_contiguous(
            pages,
            phys_addr,
            virt_addr,
            flags,
            &mut paging_ctl.frame_allocator,
        )?;

        Ok(Self {
            address: AddressPair(virt_addr, phys_addr),
            size,
            flags,
            free_after_use: false,
        })
    }

    pub fn read<T>(&self, offset: usize) -> Result<T, MemoryError> {
        self.handle_errors::<T>(false, offset)?;
        Ok(unsafe { ptr::read(self.address.0.add(offset as u64).as_ptr()) })
    }

    pub fn read_volatile<T>(&self, offset: usize) -> Result<T, MemoryError> {
        self.handle_errors::<T>(false, offset)?;
        Ok(unsafe { ptr::read_volatile(self.address.0.add(offset as u64).as_ptr()) })
    }

    pub fn write<T>(&self, offset: usize, value: T) -> Result<(), MemoryError> {
        self.handle_errors::<T>(true, offset)?;
        unsafe { ptr::write(self.address.0.add(offset as u64).as_mut_ptr(), value) };
        Ok(())
    }

    pub fn write_volatile<T>(&self, offset: usize, value: T) -> Result<(), MemoryError> {
        self.handle_errors::<T>(true, offset)?;
        unsafe { ptr::write_volatile(self.address.0.add(offset as u64).as_mut_ptr(), value) };
        Ok(())
    }

    pub fn write_back<T, F: FnOnce(T) -> T>(&self, offset: usize, f: F) -> Result<(), MemoryError> {
        self.handle_errors::<T>(true, offset)?;
        let original_value = unsafe { ptr::read(self.address.0.add(offset as u64).as_ptr()) };
        let new_value = f(original_value);
        unsafe { ptr::write(self.address.0.add(offset as u64).as_mut_ptr(), new_value) };
        Ok(())
    }

    pub fn write_back_volatile<T, F: FnOnce(T) -> T>(
        &self,
        offset: usize,
        f: F,
    ) -> Result<(), MemoryError> {
        self.handle_errors::<T>(true, offset)?;
        let original_value =
            unsafe { ptr::read_volatile(self.address.0.add(offset as u64).as_ptr()) };
        let new_value = f(original_value);
        unsafe { ptr::write_volatile(self.address.0.add(offset as u64).as_mut_ptr(), new_value) };
        Ok(())
    }

    fn handle_errors<T>(&self, write: bool, offset: usize) -> Result<(), MemoryError> {
        if !self.flags.contains(PageTableFlags::PRESENT) {
            Err(PageNotPresent)
        } else if write && !self.flags.contains(PageTableFlags::WRITABLE) {
            Err(WriteToReadOnly)
        } else if offset + size_of::<T>() >= self.size {
            Err(OutOfBounds)
        } else {
            Ok(())
        }
    }

    pub fn pages(&self) -> usize {
        self.size.div_ceil(PAGE_SIZE)
    }
}

impl Drop for AllocatedMemory {
    fn drop(&mut self) {
        let mut paging_ctl: &mut KernelPagingController = KERNEL_PAGING_CONTROLLER
            .lock()
            .get_mut()
            .expect("Could not obtain KernelPagingController during deallocation");

        //unmap pages and report error if present
        if let Err(unmap_err) = paging_ctl.rec_page_table.unmap_contiguous(
            self.pages(),
            self.address.0,
            &mut paging_ctl.frame_allocator,
        ) {
            panic!("\n{:?}", unmap_err);
        }

        if let Err(virt_free_err) = paging_ctl
            .virt_mem_allocator
            .free(self.address.0, self.pages())
        {
            panic!("\n{:?}", virt_free_err);
        }

        if !self.free_after_use {
            return;
        }

        if let Err(virt_free_err) = paging_ctl
            .frame_allocator
            .free_contiguous(self.address.1, self.pages())
        {
            panic!("\n{:?}", virt_free_err);
        }
    }
}
