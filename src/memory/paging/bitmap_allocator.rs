use crate::memory::MemoryError;
use crate::memory::MemoryError::{AllocationError, DoubleFree, OutOfBounds};
use crate::memory::paging::frame_allocation::{FrameAllocator, LinearFrameAllocator};
use crate::memory::paging::page_table::RecursivePageTable;
use crate::memory::paging::size_in_pages;
use core::ops::Range;
use core::slice;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{VirtAddr, align_up};

pub struct BitmapAllocator {
    bitmap: &'static mut [u8],
    size: u64,
    last_allocated: u64,
}

impl BitmapAllocator {
    /// *size* is the bitmap size in bytes (entries / 8).
    /// *set_at_init* will initialize the bitmap with no free space. Manual assignment of available space is then required.
    pub fn new(
        size: usize,
        set_at_init: bool,
        frame_allocator: &mut LinearFrameAllocator,
        page_table: &mut RecursivePageTable,
    ) -> Result<BitmapAllocator, MemoryError> {
        //TODO: make this independent from the frame allocator or page table, just use an address instead
        let pages = size_in_pages(size);

        let phys = frame_allocator.alloc_contiguous(pages)?;
        let virt = VirtAddr::new(phys.as_u64() + page_table.internal_offset);
        page_table.map_contiguous(
            pages,
            phys,
            virt,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            frame_allocator,
        );

        let bitmap = unsafe { slice::from_raw_parts_mut(virt.as_mut_ptr(), size) };

        //Sets the entire bitmap according to set_at_init
        bitmap.fill(match set_at_init {
            true => 0xFF,
            false => 0,
        });

        Ok(Self {
            bitmap,
            size: size as u64 * 8,
            last_allocated: 0,
        })
    }

    #[inline(always)]
    fn in_range(&self, bit: u64) -> bool {
        bit < self.size * 8
    }

    pub fn alloc(&mut self, bits: u64) -> Result<u64, MemoryError> {
        let free_space = self.scan(bits)?;
        self.last_allocated = free_space + bits;
        self.flag_range(&(free_space..(free_space + bits)), true)?;
        Ok(free_space)
    }
    //TODO: Think about scanning per byte and allocating that,
    ///tries to find *bits* free space
    fn scan(&self, bits: u64) -> Result<u64, MemoryError> {
        if bits >= self.size {
            return Err(OutOfBounds);
        }
        //TODO: find a good threshold
        if bits > 16 {
            self.scan_bytes(bits)
        } else {
            self.scan_bits(align_up(bits, 8))
        }
    }

    fn scan_bits(&self, target: u64) -> Result<u64, MemoryError> {
        let mut current = self.last_allocated;
        while current < self.size * 8 {
            let mut sum = 0;
            let mut found = 0;
            while sum == 0 && found < target && current < self.size * 8 {
                sum += self.bitmap[(current as usize / 8) % (self.size as usize / 8)]
                    & (1 << (current % 8));
                found += 1;
                current += 1;
            }
            if sum > 0 {
                continue;
            } else {
                return Ok(current - found);
            }
        }
        Err(AllocationError)
    }

    ///scans per 8 allocations worth, leads to some fragmentation but also speed gain
    fn scan_bytes(&self, target: u64) -> Result<u64, MemoryError> {
        let mut current = align_up(self.last_allocated, 8);
        while current < self.size {
            let mut sum = 0;
            let mut found = 0;
            while sum == 0 && found < target && current < self.size {
                sum += self.bitmap[current as usize % self.size as usize];
                found += 1;
                current += 8;
            }
            if sum > 0 {
                continue;
            } else {
                return Ok(current - found * 8);
            }
        }
        Err(AllocationError)
    }

    pub fn free(&mut self, bit: u64) -> Result<(), MemoryError> {
        if bit >= self.size {
            return Err(OutOfBounds);
        } else if !self.check(bit, false)? {
            return Err(DoubleFree);
        }
        self.flag(bit, false)?;
        Ok(())
    }

    #[inline(always)]
    fn range_in_bounds(&self, range: &Range<u64>) -> bool {
        range.end <= self.size * 8
    }

    pub fn free_range(&mut self, range: &Range<u64>) -> Result<(), MemoryError> {
        if !self.check_range(range, false)? {
            return Err(DoubleFree);
        }
        self.flag_range(range, false)?;
        todo!()
    }

    pub fn check(&self, bit: u64, set: bool) -> Result<bool, MemoryError> {
        if !self.in_range(bit) {
            return Err(OutOfBounds);
        }
        let byte = self.bitmap[bit as usize / 8];
        let bit = 1 << (bit % 8);
        Ok(set == ((byte & bit) != 0))
    }

    pub fn check_range(&self, range: &Range<u64>, set: bool) -> Result<bool, MemoryError> {
        let mut check = true;
        if !self.range_in_bounds(range) {
            return Err(OutOfBounds);
        }
        if range.end - range.start < 8 {
            for bit in range.start..range.end {
                check = check && self.check(bit, set)?;
            }
            return Ok(check);
        }

        let byte_aligned_start = range.start + (8 - range.start % 8) % 8;

        let byte_aligned_end = range.end - range.end % 8;

        for bit in range.start..byte_aligned_start {
            check = check && self.check(bit, set)?;
        }

        let test = match set {
            true => 0xFF,
            false => 0,
        };

        let check_range = byte_aligned_start as usize / 8..byte_aligned_end as usize / 8;

        //TODO: find a better way to do this
        for byte in self.bitmap[check_range].iter() {
            check = check && byte == &test;
        }

        for bit in byte_aligned_end..range.end {
            check = check && self.check(bit, set)?;
        }

        Ok(check)
    }

    pub fn flag(&mut self, bit: u64, flag: bool) -> Result<(), MemoryError> {
        if !self.in_range(bit) {
            return Err(OutOfBounds);
        }
        let byte = &mut self.bitmap[bit as usize / 8];
        *byte |= (flag as u8) << (bit % 8);
        Ok(())
    }

    pub fn flag_range(&mut self, range: &Range<u64>, flag: bool) -> Result<(), MemoryError> {
        if !self.range_in_bounds(range) {
            return Err(OutOfBounds);
        }
        if range.end - range.start < 8 {
            for bit in range.start..range.end {
                self.flag(bit, flag)?;
            }
            return Ok(());
        }

        let byte_aligned_start = range.start + (8 - range.start % 8) % 8;

        let byte_aligned_end = range.end - range.end % 8;

        //flag unaligned starting bits
        for bit in range.start..byte_aligned_start {
            self.flag(bit, flag)?;
        }

        let fill_range = byte_aligned_start as usize / 8..byte_aligned_end as usize / 8;

        (&mut self.bitmap[fill_range]).fill(match flag {
            true => 0xFF,
            false => 0,
        });

        //flag unaligned trailing bits
        for bit in byte_aligned_end..range.end {
            self.flag(bit, flag)?;
        }

        Ok(())
    }

    //TODO: add mapper utility
}
