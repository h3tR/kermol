use crate::PANIC_LEVEL;
use crate::display::linear_framebuffer::LinearFramebuffer;
use crate::display::vga_text_emulation::{VgaColor, get_text_buffer_dimensions, put_char, scroll};
use crate::limine_requests::FRAMEBUFFER_REQUEST;
use core::fmt;
use core::fmt::Write;
use core::sync::atomic::Ordering::SeqCst;
use limine_protocol_for_rust::requests::LimineRequest;
use spin::mutex::Mutex;
use spin::once::Once;
use x86_64::instructions::interrupts;

const TAB_SIZE: usize = 4;

pub struct VGAWriter {
    lfb: LinearFramebuffer,
    column: usize,
    row: usize,
    pub default_text: u32,
    pub default_background: u32,
    scrolling: bool,
}

impl<'a> VGAWriter {
    pub fn new(
        lfb: LinearFramebuffer,
        scrolling: bool,
        default_text: u32,
        default_background: u32,
    ) -> Self {
        Self {
            lfb,
            column: 0,
            row: 0,
            default_text,
            default_background,
            scrolling,
        }
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\t' => {
                for _ in 0..TAB_SIZE {
                    self.write_char(' ')
                }
            }
            '\n' => self.new_line(),
            _ => {
                put_char(
                    &mut self.lfb,
                    self.column,
                    self.row,
                    self.default_text,
                    self.default_background,
                    c as u8,
                );
                self.column += 1;
            }
        }
        if self.column >= get_text_buffer_dimensions(&self.lfb).0 {
            self.new_line();
        }
    }

    pub fn write_string(&mut self, string: &str) {
        for c in string.chars() {
            self.write_char(c);
        }
    }

    fn new_line(&mut self) {
        self.column = 0;
        if self.row >= get_text_buffer_dimensions(&self.lfb).1 - 1 {
            if self.scrolling {
                scroll(&mut self.lfb, 1, self.default_background);
            } else {
                self.row = 0;
            }
        } else {
            self.row += 1;
        }
    }

    pub fn set_position(&mut self, row: usize, column: usize) {
        self.row = row;
        self.column = column;
    }

    pub fn get_position(&self) -> (usize, usize) {
        (self.row, self.column)
    }
}

impl Write for VGAWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub static KWRITER: Once<Mutex<VGAWriter>> = Once::new();

pub fn init_kwriter() {
    let framebuffer_resp = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("Bootloader provided no response to the framebuffer request");
    let framebuffers = framebuffer_resp.get_framebuffers();
    let framebuffer = framebuffers.get(0).expect("no framebuffers");

    let linear_framebuffer =
        LinearFramebuffer::new(framebuffer).expect("couldn't initialize linear framebuffer");

    KWRITER.call_once(|| {
        Mutex::new(VGAWriter::new(
            linear_framebuffer,
            true,
            VgaColor::LightGray as u32,
            VgaColor::Black as u32,
        ))
    });
    PANIC_LEVEL.fetch_add(1, SeqCst); //Allow the panic handler to use kwriter to output whatever it needs directly to the screen
}

#[doc(hidden)]
pub fn _kprint(args: fmt::Arguments) {
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        KWRITER
            .get()
            .expect("Tried to kprint before setting up K_WRITER")
            .lock()
            .write_fmt(args)
            .unwrap();
    });
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ($crate::display::vga_text_writer::_kprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! kprintln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::kprint!("{}\n", format_args!($($arg)*)));
}

pub fn kwriter_set_color(color: u32) {
    interrupts::without_interrupts(|| {
        let mut kw = KWRITER
            .get()
            .expect("Tried to access before setting up K_WRITER")
            .lock();
        kw.default_text = color;
    });
}

pub fn kwriter_set_bg(background: u32) {
    interrupts::without_interrupts(|| {
        let mut kw = KWRITER
            .get()
            .expect("Tried to access before setting up K_WRITER")
            .lock();
        kw.default_background = background;
    });
}
