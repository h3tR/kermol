use crate::memory::MemoryError::{
    EmptyAllocation, LockedAllocator, OutOfBounds, PageNotPresent, WriteToReadOnly,
};
use crate::memory::{AddressPair, MemoryError, get_frame_count};
use crate::return_if_none;
use alloc::vec::Vec;
use core::ops::Add;
use core::ptr;
use x86_64::PhysAddr;
use x86_64::structures::paging::{PageTableFlags, PhysFrame};
use crate::memory::paging::{FRAME_ALLOCATOR, PAGE_SIZE};
use crate::memory::paging::paging::{map, unmap};

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
        let mut allocator = FRAME_ALLOCATOR.lock();
        let frames = return_if_none!(allocator.get_mut(), LockedAllocator)
            .alloc(get_frame_count(size) as u64)?;
        drop(allocator);

        let phys_addr = frames
            .first()
            .expect("frame allocator provided empty frame vec")
            .start_address();

        let virt_addr = map(frames, flags)?;

        Ok(Self {
            address: AddressPair(virt_addr, phys_addr),
            size,
            flags,
            free_after_use: true,
        })
    }

    pub fn at(
        phys_addr: PhysAddr,
        size: usize,
        flags: PageTableFlags,
    ) -> Result<Self, MemoryError> {
        if size == 0 {
            return Err(EmptyAllocation);
        }

        let mut allocator = FRAME_ALLOCATOR.lock();

        let frames = return_if_none!(allocator.get_mut(), LockedAllocator).alloc_at(
            PhysFrame::containing_address(phys_addr),
            get_frame_count(size) as u64,
        )?;
        drop(allocator);

        Ok(Self {
            address: AddressPair(map(frames, flags)?, phys_addr),
            size,
            flags,
            free_after_use: true,
        })
    }

    pub fn leaking(size: usize, flags: PageTableFlags) -> Result<Self, MemoryError> {
        if size == 0 {
            return Err(EmptyAllocation);
        }
        let mut allocator = FRAME_ALLOCATOR.lock();

        let frames = return_if_none!(allocator.get_mut(), LockedAllocator)
            .alloc(get_frame_count(size) as u64)?;
        drop(allocator);

        let phys_addr = frames
            .first()
            .expect("frame allocator provided empty frame vec")
            .start_address();

        Ok(Self {
            address: AddressPair(map(frames, flags)?, phys_addr),
            size,
            flags,
            free_after_use: false,
        })
    }

    pub fn mmio(phys_addr: PhysAddr, size: usize, write_trough: bool) -> Result<Self, MemoryError> {
        if size == 0 {
            return Err(EmptyAllocation);
        }

        let mut frames = Vec::new();
        for frame in 0..size / PAGE_SIZE {
            frames.push(PhysFrame::containing_address(
                phys_addr.add((frame * PAGE_SIZE) as u64),
            ));
        }

        let mut flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
        if write_trough {
            flags |= PageTableFlags::WRITE_THROUGH;
        }

        Ok(Self {
            address: AddressPair(map(frames, flags)?, phys_addr),
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
}

impl Drop for AllocatedMemory {
    fn drop(&mut self) {
        if !self.free_after_use {
            return;
        }
        //unmap pages
        let unmap = unmap(self.address.0, get_frame_count(self.size) as u64);
        if unmap.is_err() {
            panic!("\n{:?}", unmap.unwrap_err());
        }

        //deallocate frames
        for frame in 0..get_frame_count(self.size) {
            FRAME_ALLOCATOR
                .lock()
                .get_mut()
                .expect("Frame Allocator unavailable")
                .free(PhysFrame::containing_address(
                    self.address.1.add((frame * PAGE_SIZE) as u64),
                ))
                .unwrap();
        }
    }
}
