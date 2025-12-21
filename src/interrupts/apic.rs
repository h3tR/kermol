/*use crate::memory::paging::PAGE_SIZE;
use crate::memory::AllocatedMemory;
use alloc::string::String;
use raw_cpuid::CpuId;
use spin::{Mutex, Once};
use x86_64::registers::model_specific::Msr;
use x86_64::PhysAddr;

const LAPIC_BASE_MSR: u32 = 0x1B;

const LAPIC_SPURIOUS_INTERRUPT_VECTOR_REGISTER: u64 = 0xF0;
const LAPIC_IN_SERVCE_REGISTER: u64 = 0x100;
const LAPIC_EOI_REGISTER: u64 = 0xB0;

static LAPIC: Mutex<Once<AllocatedMemory>> = Mutex::new(Once::new());


pub fn set_up_apic() -> Result<(),String> {
    if !CpuId::new().get_feature_info().unwrap().has_apic() {
        panic!("CPU does not support APIC!");
    }
    // Read APIC base MSR
    let apic_base = get_apic_base();
    const APIC_ENABLE: u64 = 0x800;
    // Enable LAPIC if not already enabled
    if (apic_base & APIC_ENABLE) != APIC_ENABLE {
        unsafe { Msr::new(0x1B).write(apic_base | APIC_ENABLE) };
    }

    const APIC_SVR_ENABLE: u32 = 1 << 8;
    const APIC_VECTOR: u32 = 0xFF;

    let lapic = AllocatedMemory::mmio(
        PhysAddr::new(get_apic_base()),
        PAGE_SIZE,
        false)
        .expect("Could not allocate LAPIC memory");


    lapic.write_volatile(LAPIC_SPURIOUS_INTERRUPT_VECTOR_REGISTER as usize,APIC_VECTOR | APIC_SVR_ENABLE)?;

    LAPIC.lock().call_once(||lapic);

    Ok(())
}

pub fn acknowledge_apic() -> Result<(),String> {
    LAPIC.lock().get_mut().unwrap().write_volatile(LAPIC_EOI_REGISTER as usize,0)
}

pub fn get_apic_base() -> u64 {
    unsafe { Msr::new(LAPIC_BASE_MSR).read() & 0xFFFFF000 }
}
*/
