use std::mem;

use ash::vk;

#[repr(u32)]
pub enum Format {
    Rgba8Unorm = 37,
    Rgba8Srgb = 43,
    Bgra8Unorm = 44,
    Bgra8Srgb = 50,
}

impl TryFrom<vk::Format> for Format {
    type Error = ();

    fn try_from(format: vk::Format) -> Result<Self, Self::Error> {
        use Format::*;

        Ok(match format {
            vk::Format::R8G8B8A8_UNORM => Rgba8Unorm,
            vk::Format::R8G8B8A8_UNORM => Rgba8Srgb,
            vk::Format::B8G8R8A8_UNORM => Bgra8Unorm,
            vk::Format::B8G8R8A8_SRGB => Bgra8Srgb,
            _ => Err(())?,
        })
    }
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        use Format::*;

        match format {
            Rgba8Unorm => Self::R8G8B8A8_UNORM,
            Rgba8Srgb => Self::R8G8B8A8_SRGB,
            Bgra8Unorm => Self::B8G8R8A8_UNORM,
            Bgra8Srgb => Self::B8G8R8A8_SRGB,
        }
    }
}
