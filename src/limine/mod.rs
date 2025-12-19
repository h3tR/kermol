use core::slice::from_raw_parts;

#[unsafe(no_mangle)]
static LIMINE_BASE_REVISION: [u64; 3] = [ 0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 4 ];

#[unsafe(no_mangle)]
static LIMINE_REQUEST_START_MARKER: [u64; 4] = [ 0xf6b8f4b39de7d1ae, 0xfab91a6940fcb9cf, 0x785c6ed015d3e316, 0x181e920a7852b9d9 ];

#[unsafe(no_mangle)]
static LIMINE_REQUEST_END_MARKER: [u64; 2] = [ 0xadc0e0531bb10d03, 0x9572709f31764c62 ];




struct LimineReqId {
    common_magic_1: u64,
    common_magic_2: u64,
    other: [u64; 2]
}

impl LimineReqId {
    fn new(other: [u64; 2]) -> Self {
        Self {
            common_magic_1: 0xc7b1dd30df4c8b88,
            common_magic_2: 0x0a82e883a194f07b,
            other
        }
    }
}

macro_rules! gen_get_response {
    ($for:ty, $get:ty) => {
        impl $for {
            fn get_response(&self) -> Option<&$get> {
                unsafe {
                    self.resp.as_ref()
                }
            }
        }
    };
}


struct MemoryMapRequest {
    id: LimineReqId,
    revision: u64,
    resp: *const MemoryMapResponse
}

gen_get_response!(MemoryMapRequest, MemoryMapResponse);


struct MemoryMapResponse {
    revision: u64,
    entry_count: u64,
    entries: *const MemoryMapEntry
}

impl MemoryMapResponse {
    fn get_entries(&self) -> &[MemoryMapEntry] {
        unsafe {
            from_raw_parts(self.entries, self.entry_count as usize)
        }
    }
}


struct MemoryMapEntry {
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




