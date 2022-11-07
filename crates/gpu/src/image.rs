use crate::prelude::*;

use ash::vk;

use bitflags::bitflags;

pub(crate) enum InternalImage {
    Managed {
        image: vk::Image,
        memory: Memory,
        view: vk::ImageView,
    },
    Swapchain {
        image: vk::Image,
        view: vk::ImageView,
    },
}
impl InternalImage {
    pub(crate) fn get_image(&self) -> vk::Image {
        match self {
            Self::Managed { image, .. } => *image,
            Self::Swapchain { image, .. } => *image,
        }
    }
    pub(crate) fn get_image_view(&self) -> vk::ImageView {
        match self {
            Self::Managed { view, .. } => *view,
            Self::Swapchain { view, .. } => *view,
        }
    }
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

#[derive(Clone, Copy)]
pub enum ImageLayout {
    Undefined,
    General,
    ColorAttachmentOptimal,
    DepthStencilAttachmentOptimal,
    Present,
}

impl From<ImageLayout> for vk::ImageLayout {
    fn from(layout: ImageLayout) -> Self {
        match layout {
            ImageLayout::Undefined => Self::UNDEFINED,
            ImageLayout::General => Self::GENERAL,
            ImageLayout::ColorAttachmentOptimal => Self::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::DepthStencilAttachmentOptimal => Self::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ImageLayout::Present => Self::PRESENT_SRC_KHR,
        }
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
