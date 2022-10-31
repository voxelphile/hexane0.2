use crate::prelude::*;

use std::ops;

use ash::extensions::{ext, khr};
use ash::{vk, Entry, Instance};

pub fn default_surface_format_selector(format: Format) -> usize {
    use Format::*;
    match format {
        Rgba8Srgb => 90,
        Rgba8Unorm => 80,
        Bgra8Srgb => 70,
        Bgra8Unorm => 60,
        _ => 0,
    }
}

pub trait SurfaceFormatSelector = ops::Fn(Format) -> usize;

#[derive(Clone, Copy)]
pub enum PresentMode {
    DoNotWaitForVBlank,
    TripleBufferWaitForVBlank,
    DoubleBufferWaitForVBlank,
    DoubleBufferWaitForVBlankRelaxed,
}

impl From<PresentMode> for vk::PresentModeKHR {
    fn from(present_mode: PresentMode) -> Self {
        use PresentMode::*;

        match present_mode {
            DoNotWaitForVBlank => Self::IMMEDIATE,
            TripleBufferWaitForVBlank => Self::MAILBOX,
            DoubleBufferWaitForVBlank => Self::FIFO,
            DoubleBufferWaitForVBlankRelaxed => Self::FIFO_RELAXED,
        }
    }
}

pub struct SwapchainInfo<'a> {
    pub present_mode: PresentMode,
    pub image_usage: ImageUsage,
    pub width: u32,
    pub height: u32,
    pub surface_format_selector: &'a dyn SurfaceFormatSelector,
    pub old_swapchain: Option<Swapchain<'a>>,
    pub debug_name: &'a str,
}

impl Default for SwapchainInfo<'_> {
    fn default() -> Self {
        Self {
            present_mode: PresentMode::DoNotWaitForVBlank,
            image_usage: ImageUsage::TRANSFER_DST,
            width: 960,
            height: 540,
            surface_format_selector: &default_surface_format_selector,
            old_swapchain: None,
            debug_name: "Swapchain",
        }
    }
}

pub struct Swapchain<'a> {
    pub(crate) device: &'a Device<'a>,
    pub(crate) loader: khr::Swapchain,
    pub(crate) handle: vk::SwapchainKHR,
}

impl Swapchain<'_> {
    pub fn acquire(&mut self) -> Image {
        todo!()
    }
}
