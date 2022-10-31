use ash::vk;

use bitflags::bitflags;

pub(crate) struct InternalImage {
    pub(crate) image: vk::Image,
}

#[derive(Clone, Copy)]
pub struct Image(pub(crate) usize);

impl From<Image> for usize {
    fn from(handle: Image) -> Self {
        handle.0
    }
}

impl From<usize> for Image {
    fn from(handle: usize) -> Self {
        Self(handle)
    }
}

bitflags! {
    pub struct ImageUsage: u32 {
        const TRANSFER_SRC = 0x00000001;
        const TRANSFER_DST = 0x00000002;
        const SAMPLED = 0x00000004;
        const STORAGE = 0x00000008;
        const COLOR = 0x00000010;
        const DEPTH_STENCIL = 0x00000020;
        const TRANSIENT = 0x00000040;
        const INPUT = 0x00000080;
    }
}
