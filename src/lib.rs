#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
extern crate alloc;
use core::fmt::Write;

mod display;
mod interrupts;
mod limine_requests;
mod memory;
mod serial;
mod util;

use crate::display::vga_text_emulation::VgaColor;
use crate::display::vga_text_writer::{KWRITER, init_kwriter};
use crate::interrupts::load_idt;
use crate::limine_requests::BOOTLOADER_INFO_REQUEST;
use crate::memory::gdt::init_gdt;
use crate::memory::init_memory;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU8, Ordering};
use limine_protocol_for_rust::requests::LimineRequest;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    //TODO: get stack top
    kernel_main()
}

fn kernel_main() -> ! {
    init_kwriter();

    let bootloader_info_resp = BOOTLOADER_INFO_REQUEST
        .get_response()
        .expect("Bootloader Info was not provided");

    kprintln!(
        "Kermol was loaded using {} {}",
        bootloader_info_resp.get_name(),
        bootloader_info_resp.get_version()
    );

    init_gdt();
    kprintln!("Global Descriptor Table initialized");

    load_idt();
    kprintln!("Interrupt Descriptor Table loaded, Exceptions are now enabled");

    //TODO: pass stack top to memory_init
    if let Some(memory_error) = init_memory().err() {
        panic!("{:?}", memory_error);
    }
    //TODO: create new stack, jump there and free the old one

    serial_println!("Hi");

    x86_64::instructions::hlt();
    loop {}
}

///This variable determines how far the kernel got into setting itself up and thus what logging/displaying features are available to use.
pub static PANIC_LEVEL: AtomicU8 = AtomicU8::new(0);

#[panic_handler]
#[doc(hidden)]
fn panic_handler(info: &PanicInfo) -> ! {
    let panic_lvl = PANIC_LEVEL.load(Ordering::Relaxed);
    match panic_lvl {
        0 => serial_println!("{}", info),
        1 => {
            serial_println!("{}", info);
            let mut panic_writer = unsafe { KWRITER.get_unchecked() }.lock();
            panic_writer.default_text = VgaColor::LightRed as u32;
            writeln!(panic_writer, "{}", info).unwrap();
        }
        _ => (), //should not be reachable
    }

    x86_64::instructions::hlt();
    loop {}
}
