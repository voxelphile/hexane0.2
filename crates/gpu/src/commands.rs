use crate::buffer::Buffer;

use std::ops;

pub struct Commands {

}

pub struct BufferWrite<'a, T: Copy> {
    buffer: Buffer,
    offset: usize,
    source: &'a [T],
}

impl Commands {
    pub fn write_buffer<'a, T: Copy>(&mut self, write: BufferWrite<'a, T>) {
        todo!()
    }
}
