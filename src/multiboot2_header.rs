#[unsafe(link_section = ".multiboot2_header")]
#[unsafe(no_mangle)]
pub static MULTIBOOT_HEADER: MultibootHeader = MultibootHeader::new();

#[repr(C, packed)]
pub struct MultibootHeader {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    information_request: MultiBootTag,
    requests: [u32; 5],
   /* framebuffer: MultiBootTag,
    width: u32,
    height: u32,
    depth: u32,*/
    efi_boot_services: MultiBootTag,
    efi_amd64_entry: MultiBootTag,
    entry_addr: u64,
    end_tag: MultiBootTag,
    // Tags follow here...
}

impl MultibootHeader {
    const fn new() -> Self {
        let magic = 0xE85250D6;  // Multiboot2 magic
        let architecture = 0;    // 0 = i386, 4 = MIPS
        let header_length = size_of::<Self>() as u32 + (size_of::<Self>() as u32) % 8;
        let checksum = 0xFFFFFFFF - (magic + architecture + header_length) + 1;

        Self {
            magic,
            architecture,
            header_length,
            checksum,
            information_request: MultiBootTag::new(1, 5 * 4),
            requests: REQUESTS,
            efi_boot_services: MultiBootTag::new(7,0),
            efi_amd64_entry: MultiBootTag::new(12, 8),
            entry_addr: 0x100000,
            end_tag: MultiBootTag::new(0, 0),
        }
    }
}



///for more info visit [https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Header-tags]
#[repr(C)]
struct MultiBootTag{
    tag_type: u16,
    flags: u16,
    size: u32,
}

impl MultiBootTag {
    const fn new(tag_type: u16, extra_size_bytes: u32) -> Self {
        Self {
            tag_type,
            flags: 0,
            size: size_of::<Self>() as u32 + extra_size_bytes,
        }
    }
}

const REQUESTS: [u32; 5] = [
    4,  // Basic memory info
    6,  // Memory map (required for setting up memory management
    8,  // Boot device
    9,  // Command line
    10, // Modules
];