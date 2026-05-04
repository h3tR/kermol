use crate::util::KIBIBYTE;
use spin::once::Once;
use x86_64::instructions::segmentation::{Segment, CS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use crate::kprintln;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<GlobalDescriptorTable> = Once::new();


pub fn init_gdt() {
    TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            //we set up a different mall stack here for use in the double fault handler
            const STACK_SIZE: usize = 4 * KIBIBYTE;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &raw const STACK });
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    });

    let mut gdt = GlobalDescriptorTable::new();

    let code_segment = gdt.append(Descriptor::kernel_code_segment());
    let task_state_segment = gdt.append(Descriptor::tss_segment(&TSS.get().unwrap()));

    GDT.call_once(|| gdt);
    GDT.get().unwrap().load();

    unsafe {
        CS::set_reg(code_segment);
        load_tss(task_state_segment);
    }
}
