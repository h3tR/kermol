use crate::util::KIBIBYTE;
use limine_protocol_for_rust::requests::bootloader_info::BootloaderInfoRequest;
use limine_protocol_for_rust::requests::executable_address::ExecutableAddressRequest;
use limine_protocol_for_rust::requests::framebuffer::FramebufferRequest;
use limine_protocol_for_rust::requests::hhdm::HigherHalfDirectMapRequest;
use limine_protocol_for_rust::requests::memory_map::MemoryMapRequest;
use limine_protocol_for_rust::requests::smbios::SmbiosRequest;
use limine_protocol_for_rust::requests::stack_size::StackSizeRequest;
use limine_protocol_for_rust::use_base_revision;

const REVISION: u64 = 4;

#[used]
#[unsafe(link_section = ".limine_reqs")]
static LIMINE_BASE_REVISION: [u64; 4] = use_base_revision(REVISION);

#[used]
#[unsafe(link_section = ".limine_req_start")]
static LIMINE_REQUEST_START_MARKER: [u64; 4] = limine_protocol_for_rust::REQUEST_START_MARKER;

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new(REVISION);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new(REVISION);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static BOOTLOADER_INFO_REQUEST: BootloaderInfoRequest = BootloaderInfoRequest::new(REVISION);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static HHDM_REQUEST: HigherHalfDirectMapRequest = HigherHalfDirectMapRequest::new(REVISION);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static KERNEL_ADDRESS_REQUEST: ExecutableAddressRequest =
    ExecutableAddressRequest::new(REVISION);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static SMBIOS_REQUEST: SmbiosRequest = SmbiosRequest::new(REVISION);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static STACK_SIZE_REQUEST: StackSizeRequest =
    StackSizeRequest::new(REVISION, 64 * KIBIBYTE as u64);
#[used]
#[unsafe(link_section = ".limine_req_end")]
static LIMINE_REQUEST_END_MARKER: [u64; 2] = limine_protocol_for_rust::REQUEST_END_MARKER;
