use crate::{kprintln, serial_println};
use core::{ptr, slice};
use limine_protocol_for_rust::requests::framebuffer::Framebuffer;

pub struct LinearFramebuffer {
    ///Interpret as (Width, Height)
    pub dimensions: (usize, usize),
    buffer: &'static mut [u32],
}

#[derive(Debug)]
pub enum LinearFramebufferError {
    UnsupportedPixelDepth,
    OutOfBounds,
}

impl LinearFramebuffer {
    pub fn new(framebuffer: &Framebuffer) -> Result<Self, LinearFramebufferError> {
        if framebuffer.bpp != 32 {
            return Err(LinearFramebufferError::UnsupportedPixelDepth);
        }
        Ok(Self {
            dimensions: (framebuffer.width as usize, framebuffer.height as usize),
            buffer: unsafe {
                slice::from_raw_parts_mut(
                    framebuffer.address as *mut u32,
                    framebuffer.width as usize * framebuffer.height as usize,
                )
            },
        })
    }

    ///Color needs to be the right depth, otherwise it will be automatically truncated (not scaled). This will result in incorrect color values.
    ///This function is incredibly slow. You probably shouldn't use this.
    pub fn plot(&mut self, x: usize, y: usize, color: u32) {
        let offset = x + y * self.dimensions.0;
        self.buffer[offset] = color;
    }

    pub fn plot_glyph(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        glyph_bitmap: &[u8],
        color: u32,
        background: u32,
    ) -> Result<(), LinearFramebufferError> {
        if x + width > self.dimensions.0 || y + height > self.dimensions.1 {
            return Err(LinearFramebufferError::OutOfBounds);
        }

        let mut current_x = 0;
        let mut current_y = 0;
        let mut glyph_iterator = glyph_bitmap.iter();
        'outer: while let Some(glyph_byte) = glyph_iterator.next() {
            for bit in 0..8 {
                let mut used_color = color;
                if (glyph_byte >> (7 - bit)) & 1u8 == 0u8 {
                    used_color = background;
                }
                self.plot(current_x + x, current_y + y, used_color);
                current_x += 1;
                if current_x >= width {
                    current_x = 0;
                    current_y += 1;

                    if current_y >= height {
                        break 'outer;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    pub fn scroll(&mut self, layers: usize, color: u32) {
        unsafe {
            ptr::copy(
                self.buffer.as_ptr().add(layers * self.dimensions.0),
                self.buffer.as_mut_ptr(),
                self.dimensions.0 * (self.dimensions.1 - layers),
            );
        }

        for i in 0..layers * self.dimensions.0 {
            unsafe {
                self.buffer
                    .as_mut_ptr()
                    .add(self.dimensions.0 * (self.dimensions.1 - layers) + i)
                    .write(color);
            }
        }
    }
}
