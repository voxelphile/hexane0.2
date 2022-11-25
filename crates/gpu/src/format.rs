use std::mem;

use ash::vk;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    #[default]
    Undefined,
    R32Uint,
    Rgba8Uint,
    Rgb32Uint,
    Rgba8Unorm,
    Rgba8Srgb,
    Bgra8Unorm,
    Bgra8Srgb,
    D32Sfloat,
}

impl From<Format> for vk::ImageAspectFlags {
    fn from(format: Format) -> Self {
        match format {
            Format::D32Sfloat => vk::ImageAspectFlags::DEPTH,
            _ => vk::ImageAspectFlags::COLOR,
        }
    }
}

impl TryFrom<vk::Format> for Format {
    type Error = ();

    fn try_from(format: vk::Format) -> Result<Self, Self::Error> {
        use Format::*;

        Ok(match format {
            vk::Format::UNDEFINED => Undefined,
            vk::Format::R32_UINT => R32Uint,
            vk::Format::R8G8B8A8_UINT => Rgba8Uint,
            vk::Format::R32G32B32_UINT => Rgb32Uint,
            vk::Format::R8G8B8A8_UNORM => Rgba8Unorm,
            vk::Format::R8G8B8A8_UNORM => Rgba8Srgb,
            vk::Format::B8G8R8A8_UNORM => Bgra8Unorm,
            vk::Format::B8G8R8A8_SRGB => Bgra8Srgb,
            vk::Format::D32_SFLOAT => D32Sfloat,
            _ => Err(())?,
        })
    }
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        use Format::*;

        match format {
            Undefined => Self::UNDEFINED,
            R32Uint => Self::R32_UINT,
            Rgb32Uint => Self::R32G32B32_UINT,
            Rgba8Uint => Self::R8G8B8A8_UINT,
            Rgba8Unorm => Self::R8G8B8A8_UNORM,
            Rgba8Srgb => Self::R8G8B8A8_SRGB,
            Bgra8Unorm => Self::B8G8R8A8_UNORM,
            Bgra8Srgb => Self::B8G8R8A8_SRGB,
            D32Sfloat => Self::D32_SFLOAT,
        }
    }
}
