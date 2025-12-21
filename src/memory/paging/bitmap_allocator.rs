use core::slice;
use crate::memory::MemoryError;
use crate::memory::MemoryError::{AllocationError, DoubleFree, OutOfBounds};

pub struct BitmapAllocator {
    bitmap: &'static mut [u8],
    size: u64,
    last_allocated: u64,
}

impl BitmapAllocator {
    pub fn new(address: *mut u8, size: usize, start_offset: u64) -> Self{
        let bitmap = unsafe { slice::from_raw_parts_mut(address, size) };
        bitmap.fill(0);
        
        Self {
            bitmap,
            size: size as u64,
            last_allocated: start_offset,
        }
    }
    
    fn is_set(&self, bit: u64) -> bool {
        let byte = self.bitmap[bit as usize / 8];
        let bit = 1 << (bit % 8);
        (byte & bit) != 0
    }

    ///Should only be used for initialization of the bitmap
    pub fn set(&mut self, bit: u64) {
        let byte = &mut self.bitmap[bit as usize / 8];
        *byte |= 1 << (bit % 8);
    }

    fn clear(&mut self, bit: u64) {
        let byte = &mut self.bitmap[bit as usize / 8];
        *byte &= !(1 << (bit % 8));
    }

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
                    self.set(start_bit + bit)
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
            self.set(start_bit + bit)
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
        self.clear(bit);
        Ok(())
    }
}
