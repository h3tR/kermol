use crate::kprintln;
use crate::util::MEBIBYTE;
use linked_list_allocator::LockedHeap;
use x86_64::VirtAddr;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub(super) const KERNEL_HEAP_SIZE: usize = 4 * MEBIBYTE;

pub(super) fn init_heap(address: VirtAddr) {
    unsafe {
        ALLOCATOR
            .lock()
            .init(address.as_u64() as usize, KERNEL_HEAP_SIZE);
    }

    kprintln!("Heap Initialized");
}
