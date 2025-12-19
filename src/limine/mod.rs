pub mod requests;

use core::mem::MaybeUninit;
use core::slice::from_raw_parts;

#[repr(C, align(8))]
struct LimineReqId {
    common_magic: [u64; 2],
    other: [u64; 2]
}

impl LimineReqId {
    const fn new(other: [u64; 2]) -> Self {
        Self {
            common_magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b],
            other
        }
    }
}

macro_rules! gen_get_response {
    ($get:ty) => {
        pub fn get_response(&self) -> Option<&$get> {
            unsafe {
                if self.resp.assume_init() == 0 {
                   return None
                }
                (self.resp.assume_init() as *const $get).as_ref()
            }
        }
    };
}


#[repr(C, align(8))]
pub struct MemoryMapRequest {
    id: LimineReqId,
    revision: u64,
    resp: MaybeUninit<usize>
}

impl MemoryMapRequest {
    const fn new(revision: u64) -> Self {
        Self {
            id: LimineReqId::new([0x67cf3d9d378a806f, 0xe304acdfc50c3c62]),
            revision,
            resp: MaybeUninit::uninit()
        }
    }

    gen_get_response!(MemoryMapResponse);



}


#[repr(C, align(8))]
pub struct MemoryMapResponse {
    revision: u64,
    entry_count: u64,
    entries: *const MemoryMapEntry
}

impl MemoryMapResponse {
    pub fn get_entries(&self) -> &[MemoryMapEntry] {
        unsafe {
            from_raw_parts(self.entries, self.entry_count as usize)
        }
    }
}

#[repr(C, align(8))]
pub struct MemoryMapEntry {
    base: u64,
    length: u64,
    memmap_type: u64
}

impl MemoryMapEntry {
    fn type_as_enum(&self) -> MemoryMapType {
        match self.memmap_type {
            0 => MemoryMapType::Usable,
            1 => MemoryMapType::Reserved,
            2 => MemoryMapType::AcpiReclaimable,
            3 => MemoryMapType::AcpiNvs,
            4 => MemoryMapType::BadMemory,
            5 => MemoryMapType::BootloaderReclaimable,
            6 => MemoryMapType::ExecutableAndModules,
            7 => MemoryMapType::Framebuffer,
            8 => MemoryMapType::AcpiTables,
            _ => panic!("Obtained invalid MemoryMap Type")
        }
    }
}

enum MemoryMapType {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    BootloaderReclaimable,
    ExecutableAndModules,
    Framebuffer,
    AcpiTables
}

#[repr(C, align(8))]
pub struct FramebufferRequest {
    id: LimineReqId,
    revision: u64,
    resp: MaybeUninit<usize>
}

impl FramebufferRequest {
    const fn new(revision: u64) -> Self {
        Self {
            id: LimineReqId::new([0x9d5827dcd881dd75, 0xa3148604f6fab11b]),
            revision,
            resp: MaybeUninit::uninit()
        }
    }

    gen_get_response!(FramebufferResponse);
}


#[repr(C, align(8))]
pub struct FramebufferResponse {
    revision: u64,
    entry_count: u64,
    entries: *const Framebuffer
}

impl FramebufferResponse {
    pub fn get_entries(&self) -> &[Framebuffer] {
        unsafe {
            from_raw_parts(self.entries, self.entry_count as usize)
        }
    }
}

#[repr(C, align(8))]
pub struct Framebuffer {
    pub address: usize,
    width: u64,
    height: u64,
    pub pitch: u64,
    bpp: u16,
    memory_model: u8,
    red_mask_size: u8,
    red_mask_shift: u8,
    green_mask_size: u8,
    green_mask_shift: u8,
    blue_mask_size: u8,
    blue_mask_shift: u8,
    _unused: [u8; 7],
    edid_size: u64,
    edid_address: usize,

    //Response revision 1
    mode_count: u64,
    modes: *const VideoMode
}

impl Framebuffer {
    pub fn get_modes(&self) -> &[VideoMode] {
        unsafe {
            from_raw_parts(self.modes, self.mode_count as usize)
        }
    }
}

#[repr(C)]
pub struct VideoMode {
    pitch: u64,
    width: u64,
    height: u64,
    bpp: u16,
    memory_model: u8,
    red_mask_size: u8,
    red_mask_shift: u8,
    green_mask_size: u8,
    green_mask_shift: u8,
    blue_mask_size: u8,
    blue_mask_shift: u8,
}