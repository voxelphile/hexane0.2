pub use crate::prelude::*;

use ash::vk;

use bitflags::bitflags;

bitflags! {
    pub struct BufferUsage: u32 {
        const TRANSFER_SRC = 0x00000001;
        const TRANSFER_DST = 0x00000002;
        const STORAGE = 0x00000004;
        const INDEX = 0x00000008;
        const VERTEX = 0x00000010;
    }
}

impl From<BufferUsage> for vk::BufferUsageFlags {
    fn from(usage: BufferUsage) -> Self {
        let mut result = vk::BufferUsageFlags::empty();

        if usage.contains(BufferUsage::TRANSFER_SRC) {
            result |= vk::BufferUsageFlags::TRANSFER_SRC;
        }

        if usage.contains(BufferUsage::TRANSFER_DST) {
            result |= vk::BufferUsageFlags::TRANSFER_DST;
        }

        if usage.contains(BufferUsage::STORAGE) {
            result |= vk::BufferUsageFlags::STORAGE_BUFFER;
        }

        if usage.contains(BufferUsage::INDEX) {
            result |= vk::BufferUsageFlags::INDEX_BUFFER;
        }

        if usage.contains(BufferUsage::VERTEX) {
            result |= vk::BufferUsageFlags::VERTEX_BUFFER;
        }

        result
    }
}

pub(crate) struct InternalBuffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) size: usize,
    pub(crate) usage: BufferUsage,
    pub(crate) memory: InternalMemory,
    pub(crate) debug_name: String,
}

#[derive(Clone, Copy)]
pub struct Buffer(pub(crate) usize);

impl From<Buffer> for usize {
    fn from(handle: Buffer) -> Self {
        handle.0
    }
}

impl From<usize> for Buffer {
    fn from(handle: usize) -> Self {
        Self(handle)
    }
}

pub struct BufferInfo<'a> {
    pub size: usize,
    pub memory: Memory,
    pub usage: BufferUsage,
    pub debug_name: &'a str,
}

impl Default for BufferInfo<'_> {
    fn default() -> Self {
        Self {
            size: 0,
            memory: Memory::empty(),
            usage: BufferUsage::all(),
            debug_name: "Buffer",
        }
    }
}
