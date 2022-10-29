pub struct Semaphore {}

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

pub struct TimelineSemaphoreInfo<'a> {
    pub debug_name: &'a str,
}

impl Default for TimelineSemaphoreInfo<'_> {
    fn default() -> Self {
        Self {
            debug_name: "Timeline Semaphore",
        }
    }
}
