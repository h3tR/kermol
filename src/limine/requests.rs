use crate::limine::{FramebufferRequest, MemoryMapRequest};

#[used]
#[unsafe(link_section = ".limine_reqs")]
static LIMINE_BASE_REVISION: [u64; 4] = [ 0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 4, 0 ];

#[used]
#[unsafe(link_section = ".limine_req_start")]
static LIMINE_REQUEST_START_MARKER: [u64; 4] = [ 0xf6b8f4b39de7d1ae, 0xfab91a6940fcb9cf, 0x785c6ed015d3e316, 0x181e920a7852b9d9 ];

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new(4);

#[used]
#[unsafe(link_section = ".limine_reqs")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new(4);

#[used]
#[unsafe(link_section = ".limine_req_end")]
static LIMINE_REQUEST_END_MARKER: [u64; 2] = [ 0xadc0e0531bb10d03, 0x9572709f31764c62 ];