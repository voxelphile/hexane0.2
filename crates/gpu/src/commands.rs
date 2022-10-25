use crate::buffer::Buffer;
use crate::pipeline::Pipeline;

use std::ops;

pub struct Commands {

}

pub struct BufferWrite<'a, T: Copy> {
    buffer: Buffer,
    offset: usize,
    source: &'a [T],
}

pub struct BufferCopy<'a, R: ops::RangeBounds<usize>> {
    from: &'a Buffer,
    to: &'a mut Buffer,
    range: R,
}

pub struct DrawIndexed {
    index_count: usize,
}

impl Commands {
    pub fn write_buffer<'a, T: Copy>(&mut self, write: BufferWrite<'a, T>) {
        todo!()
    }
    pub fn copy_buffer_to_buffer<'a, R: ops::RangeBounds<usize>>(&mut self, copy: BufferCopy<'a, R>) {
        todo!()
    }
    pub fn begin_render_pass(&mut self) {
        todo!()
    }
    pub fn end_render_pass(&mut self) {
        todo!()
    }
    pub fn set_pipeline(&mut self, pipeline: Pipeline) {
        todo!()
    }
    pub fn draw_indexed(&mut self, draw: DrawIndexed) {
        todo!()
    }
}
