#![feature(trait_alias)]
#![feature(let_else)]

mod context;
mod device;
mod format;
mod image;
mod swapchain;

pub use context::Info as ContextInfo;
pub use context::*;
pub use device::Info as DeviceInfo;
pub use device::*;
pub use format::*;
pub use image::*;
pub use swapchain::Info as SwapchainInfo;
pub use swapchain::*;

use std::error;
use std::fmt;
use std::result;

pub mod prelude {
    pub use crate::{
        Context, ContextInfo, Device, DeviceInfo, DeviceSelector, Error, Format, ImageUsage,
        PresentMode, Result, SurfaceFormatSelector, Swapchain, SwapchainInfo,
    };
}

#[derive(Debug)]
pub enum Error {
    Creation,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for Error {}

pub type Result<T> = result::Result<T, Error>;
