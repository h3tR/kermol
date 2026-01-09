use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode, SelectorErrorCode};

pub extern "x86-interrupt" fn division_error_handler(stack_frame: InterruptStackFrame) {
    panic!("DIVISION ERROR: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    panic!("DEBUG: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    panic!("NON-MASKABLE INTERRUPT: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    panic!("BREAKPOINT: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    panic!("OVERFLOW: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    panic!("BOUND RANGE EXCEEDED: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("INVALID OPCODE: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    panic!("DEVICE_NOT_AVAILABLE: \n{:#?}", stack_frame);
}

///Only has 4 KiB stack size so nothing major should be done here
pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("!DOUBLE FAULT: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "INVALID TASK-STATE SEGMENT: \n{:#?}\n {:?}",
        stack_frame,
        SelectorErrorCode::new(error_code).unwrap()
    );
}

pub extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "SEGMENT NOT PRESENT: \n{:#?}\n {:?}",
        stack_frame,
        SelectorErrorCode::new(error_code).unwrap()
    );
}

pub extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "STACK-SEGMENT FAULT: \n{:#?}\n {:?}",
        stack_frame,
        SelectorErrorCode::new(error_code).unwrap()
    );
}

pub extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "GENERAL PROTECTION FAULT: \n{:#?}\n{:?}",
        stack_frame,
        SelectorErrorCode::new(error_code).unwrap()
    );
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    panic!(
        "PAGE FAULT \nAccessed Address: {:?}\nError Code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
}

pub extern "x86-interrupt" fn x87_floating_point_exception_handler(
    stack_frame: InterruptStackFrame,
) {
    panic!("X87 FLOATING-POINT EXCEPTION: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "ALIGNMENT CHECK: \n{:#?}\n error code: {}",
        stack_frame, error_code
    );
}

pub extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) {
    panic!("MACHINE CHECK: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn simd_floating_point_exception_handler(
    stack_frame: InterruptStackFrame,
) {
    panic!("SIMD FLOATING-POINT EXCEPTION: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn virtualization_exception_handler(stack_frame: InterruptStackFrame) {
    panic!("VIRTUALIZATION EXCEPTION: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn control_protection_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "CONTROL PROTECTION EXCEPTION: \n{:#?}\n error code: {}",
        stack_frame, error_code
    );
}

pub extern "x86-interrupt" fn hypervisor_injection_exception_handler(
    stack_frame: InterruptStackFrame,
) {
    panic!("HYPERVISOR INJECTION EXCEPTION: \n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn vmm_communication_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "VMM COMMUNICATION EXCEPTION: \n{:#?}\n error code: {}",
        stack_frame, error_code
    );
}

pub extern "x86-interrupt" fn security_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "SECURITY EXCEPTION: \n{:#?}\n error code: {}",
        stack_frame, error_code
    );
}
