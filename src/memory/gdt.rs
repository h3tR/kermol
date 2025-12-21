use crate::util::KIBIBYTE;
use spin::once::Once;
use x86_64::VirtAddr;
use x86_64::instructions::segmentation::{CS, DS, SS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{ES, FS, GS, SegmentSelector};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use crate::memory::DOUBLE_FAULT_IST_INDEX;

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

struct Selectors {
    code_segment: SegmentSelector,
    data_segment: SegmentSelector,
    task_state_segment: SegmentSelector,
}

pub fn init_gdt() {
    TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 5 * KIBIBYTE;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe {&raw const STACK}) ;
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    });
    GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        let code_segment = gdt.append(Descriptor::kernel_code_segment());
        let data_segment = gdt.append(Descriptor::kernel_data_segment());
        let task_state_segment = gdt.append(Descriptor::tss_segment(&TSS.get().unwrap()));
        (
            gdt,
            Selectors {
                code_segment,
                data_segment,
                task_state_segment,
            },
        )
    });

    GDT.get().unwrap().0.load();

    unsafe {
        CS::set_reg(GDT.get().unwrap().1.code_segment);
        DS::set_reg(GDT.get().unwrap().1.data_segment);

        ES::set_reg(GDT.get().unwrap().1.data_segment);
        FS::set_reg(GDT.get().unwrap().1.data_segment);
        GS::set_reg(GDT.get().unwrap().1.data_segment);

        SS::set_reg(GDT.get().unwrap().1.data_segment);
        load_tss(GDT.get().unwrap().1.task_state_segment);
    }
}
