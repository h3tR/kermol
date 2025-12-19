#![no_std]
#![no_main]

mod limine;
mod serial;

use core::panic::PanicInfo;
use core::ptr;
use crate::limine::requests::{FRAMEBUFFER_REQUEST, MEMORY_MAP_REQUEST};

#[unsafe(no_mangle)]
static STACK_TOP: [u8; 16384] = [0; 16384];
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> !{
    kernel_main()
}

fn kernel_main() -> ! {

    let memory_map_resp = MEMORY_MAP_REQUEST.get_response().expect("Memory map request had no response");
    let memory_map = memory_map_resp.get_entries();

    let framebuffer_resp = FRAMEBUFFER_REQUEST.get_response().expect("Framebuffer request had no response");

    let framebuffer = framebuffer_resp.get_entries().first().unwrap();

    unsafe {
        let fb = framebuffer.address as *mut u32;
        for i in 0..100 {
            ptr::write(fb.offset( i * (framebuffer.pitch + 1) as isize),0xFFFFFF)
        }
    }
    serial_println!("Done");

    x86_64::instructions::hlt();
    loop {}
}




#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    serial_println!("{}",info);
    x86_64::instructions::hlt();
    loop {}
}
