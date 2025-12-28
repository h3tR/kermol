use crate::kprintln;
use crate::memory::MemoryError;
use crate::memory::MemoryError::{AllocationError, DoubleFree, OutOfBounds};
use core::ops::Range;
use core::slice;

pub struct BitmapAllocator {
    bitmap: &'static mut [u8],
    size: u64,
    last_allocated: u64,
}

impl BitmapAllocator {
    /// *size* is the bitmap size in bytes (entries / 8).  
    /// *set_at_init* will initalize the bitmap with no free space. Manual assignment of available space is then required.
    pub fn new(
        address: *mut u8,
        size: usize,
        set_at_init: bool,
    ) -> Result<BitmapAllocator, MemoryError> {
        let bitmap = unsafe { slice::from_raw_parts_mut(address, size) };

        //Sets the entire bitmap, essentially allocates everything
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

    //TODO: return result with possible outofbounds
    fn is_set(&self, bit: u64) -> bool {
        let byte = self.bitmap[bit as usize / 8];
        let bit = 1 << (bit % 8);
        (byte & bit) != 0
    }

    //TODO: allow using flag_range, faster...
    pub fn alloc(&mut self, bits: u64) -> Result<u64, MemoryError> {
        let mut scanned_bits: u64 = 0;
        let mut start_bit = self.last_allocated;
        while scanned_bits < self.size {
            let can_fit = {
                if start_bit + bits >= self.size {
                    start_bit = 0;
                    false
                } else {
                    let mut can_fit = true;
                    for frame in 0..bits {
                        if self.is_set(start_bit + frame % self.size) {
                            start_bit += frame + 1 % self.size;
                            scanned_bits += frame;
                            can_fit = false;
                            break;
                        }
                    }
                    can_fit
                }
            };
            if can_fit {
                for bit in 0..bits {
                    self.flag(start_bit + bit, true)
                }
                self.last_allocated = start_bit + bits;
                return Ok(start_bit);
            }
        }
        Err(AllocationError)
    }

    pub fn alloc_at(&mut self, start_bit: u64, bits: u64) -> Result<u64, MemoryError> {
        if start_bit + bits >= self.size {
            return Err(OutOfBounds);
        }
        for bit in 0..bits {
            if self.is_set(start_bit + bit % self.size) {
                return Err(AllocationError);
            }
        }
        for bit in 0..bits {
            self.flag(start_bit + bit, true)
        }
        self.last_allocated = start_bit + bits - 1;

        Ok(start_bit)
    }

    pub fn free(&mut self, bit: u64) -> Result<(), MemoryError> {
        if bit >= self.size {
            return Err(OutOfBounds);
        } else if !self.is_set(bit) {
            return Err(DoubleFree);
        }
        self.flag(bit, false);
        Ok(())
    }

    pub fn flag(&mut self, bit: u64, flag: bool) {
        let byte = &mut self.bitmap[bit as usize / 8];
        *byte |= (flag as u8) << (bit % 8);
    }

    pub fn flag_range(&mut self, range: Range<u64>, flag: bool) {
        if range.end - range.start < 8 {
            for bit in range.start..range.end {
                self.flag(bit, flag);
            }
            return;
        }

        let byte_aligned_start = range.start + (8 - range.start % 8) % 8;

        let byte_aligned_end = range.end - range.end % 8;

        //flag unaligned starting bits
        for bit in range.start..byte_aligned_start {
            self.flag(bit, flag);
        }

        let fill_range = byte_aligned_start as usize / 8..byte_aligned_end as usize / 8;

        (&mut self.bitmap[fill_range]).fill(match flag {
            true => 0xFF,
            false => 0,
        });

        //flag unaligned trailing bits
        for bit in byte_aligned_end..range.end {
            self.flag(bit, flag);
        }
    }

    //TODO: add mapper utility
}
