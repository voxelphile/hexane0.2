pub use crate::prelude::*;

use bitflags::bitflags;

#[derive(Clone, Copy)]
pub struct Buffer(pub(crate) usize);

bitflags! {
    pub struct Memory: u32 {
        const DEDICATED_MEMORY = 0x00000001;
        const CAN_ALIAS = 0x00000002;
        const HOST_ACCESS_SEQUENTIAL_WRITE = 0x00000004;
        const HOST_ACCESS_RANDOM = 0x00000008;
        const STRATEGY_MIN_MEMORY = 0x00000010;
        const STRATEGY_MIN_TIME = 0x00000020;
    }
}

pub struct BufferInfo<'a> {
    pub size: usize,
    pub memory: Memory,
    pub debug_name: &'a str,
}

impl Default for BufferInfo<'_> {
    fn default() -> Self {
        Self {
            size: 0,
            memory: Memory::empty(),
            debug_name: "Buffer",
        }
    }
}
