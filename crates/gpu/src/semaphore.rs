use crate::prelude::*;

use ash::vk;

pub struct BinarySemaphore<'a> {
    pub(crate) device: &'a Device<'a>,
    pub(crate) semaphore: vk::Semaphore,
    pub(crate) debug_name: String,
}

pub struct BinarySemaphoreInfo<'a> {
    pub debug_name: &'a str,
}

impl Default for BinarySemaphoreInfo<'_> {
    fn default() -> Self {
        Self {
            debug_name: "Binary Semaphore",
        }
    }
}

pub struct TimelineSemaphore<'a> {
    pub(crate) device: &'a Device<'a>,
    pub(crate) semaphore: vk::Semaphore,
    pub(crate) debug_name: String,
}

pub struct TimelineSemaphoreInfo<'a> {
    pub initial_value: u64,
    pub debug_name: &'a str,
}

impl Default for TimelineSemaphoreInfo<'_> {
    fn default() -> Self {
        Self {
            initial_value: 0,
            debug_name: "Timeline Semaphore",
        }
    }
}
