use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
//use crate::interrupts::apic::acknowledge_apic;
use crate::interrupts::exceptions::*;
use crate::memory::DOUBLE_FAULT_IST_INDEX;

pub mod apic;
mod exceptions;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
        pub static ref IDT: Mutex<InterruptDescriptorTable> = {
                let mut idt = InterruptDescriptorTable::new();
                //exceptions
                idt.divide_error.set_handler_fn(division_error_handler);
                idt.debug.set_handler_fn(debug_handler);
                idt.non_maskable_interrupt.set_handler_fn(nmi_handler);
                idt.breakpoint.set_handler_fn(breakpoint_handler);
                idt.overflow.set_handler_fn(overflow_handler);
                idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
                idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
                idt.device_not_available.set_handler_fn(device_not_available_handler);
                unsafe { idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(DOUBLE_FAULT_IST_INDEX); }
                idt.invalid_tss.set_handler_fn(invalid_tss_handler);
                idt.segment_not_present.set_handler_fn(segment_not_present_handler);
                idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
                idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
                idt.page_fault.set_handler_fn(page_fault_handler);
                idt.x87_floating_point.set_handler_fn(x87_floating_point_exception_handler);
                idt.alignment_check.set_handler_fn(alignment_check_handler);
                //idt.machine_check.set_handler_fn(machine_check_handler); //TODO: this doesn't work for some reason?
                idt.simd_floating_point.set_handler_fn(simd_floating_point_exception_handler);
                idt.virtualization.set_handler_fn(virtualization_exception_handler);
                idt.cp_protection_exception.set_handler_fn(control_protection_exception_handler);
                idt.hv_injection_exception.set_handler_fn(hypervisor_injection_exception_handler);
                idt.vmm_communication_exception.set_handler_fn(vmm_communication_exception_handler);
                idt.security_exception.set_handler_fn(security_exception_handler);
                //TODO: hardware interrupts


                // for i in 32..=255{
                //      idt[i].set_handler_fn(no_op);
                // }

                Mutex::new(idt)
            };
        static ref FREE_INTERRUPTS: Mutex<[bool; 256-32]> = Mutex::new([true; 256-32]);
}

pub fn load_idt() {
    let idt = IDT.lock();
    unsafe {
        idt.load_unsafe();
    };
    drop(idt);
}

pub fn init_interrupts() -> Result<(), ()> {
    // disable_pic();
    //
    // //TODO: mask unused vectors
    // set_up_apic()?;

    Ok(())
}

// fn get_free_vector() -> Option<u8> {
//         for (vector, free) in FREE_INTERRUPTS.lock().iter().enumerate() {
//                 if *free {
//                         return Some(vector as u8);
//                 }
//         }
//         None
// }
//
// ///registers a new IRQ to the IDT, reloads it and returns the IRQ vector starting at 32
// pub fn register_new_irq() -> Option<u8> {
//         let vector: Option<u8> = get_free_vector();
//         if vector.is_some() {
//                 FREE_INTERRUPTS.lock()[vector.unwrap() as usize] = false;
//                 return Some(vector.unwrap() + 32u8);
//         }
//         None
// }
//
// pub fn free_irq(vector: u8) {
//         FREE_INTERRUPTS.lock()[vector as usize - 32] = true;
//         set_handler(vector, no_op)
// }
// pub fn set_handler(
//         vector: u8,
//         handler: extern "x86-interrupt" fn(_stack_frame: InterruptStackFrame),
// ) {
//         let mut idt = IDT.lock();
//         idt[vector].set_handler_fn(handler);
//
//         unsafe { idt.load_unsafe() };
//         drop(idt);
// }
// /*
// pub extern "x86-interrupt" fn no_op(_stack_frame: InterruptStackFrame) {
//         acknowledge_apic().expect("Unable to acknowledge APIC")
// }*/
