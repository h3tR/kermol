#![no_std]
#![no_main]

mod multiboot2_header;

use core::arch::asm;
use core::panic::PanicInfo;

#[unsafe(no_mangle)]
static STACK_TOP: [u8; 16384] = [0; 16384];
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> !{

    //save boot info pointer
    let multiboot_info: u32;
    unsafe {
        asm!(
            "mov ebx, {0:e}",
            out(reg) multiboot_info,
            options(nomem, nostack)
        );
    }

    //set up stack
    unsafe {
        asm!(
            "mov {}, rsp",
            in(reg) &STACK_TOP,
            options(nomem, nostack)
        );
    }

    kernel_main(multiboot_info)
}

fn kernel_main(multiboot_info: u32) -> ! {
    x86_64::instructions::hlt();
    loop {}
}




#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}
