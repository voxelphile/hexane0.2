use crate::buffer::Buffer;
use crate::image::Image;
use crate::pipeline::Pipeline;

use std::ops;
use std::default::default;

pub struct Commands {

}

pub struct BufferWrite<'a, T: Copy> {
    buffer: &'a mut Buffer,
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

pub struct RenderPass<'a> {
    color: &'a [Attachment],
    depth: Option<Attachment>,
    render_area: RenderArea,
}

pub enum Clear {
    Color(f32, f32, f32, f32),
    Depth(f32),
    DepthStencil(f32, u32),
}

impl Default for Clear {
    fn default() -> Self {
        Self::Color(0.0, 0.0, 0.0, 1.0)
    }
}

#[derive(Default)]
pub enum LoadOp {
    #[default]
    Load,
    Clear,
    DontCare,
}

pub struct Attachment {
    image: Image,
    load_op: LoadOp,
    clear: Clear,
}

impl Default for Attachment {
    fn default() -> Self {
        Self {
            image: Image(usize::MAX),
            load_op: default(),
            clear: default(),
        }
    }
}

#[derive(Default)]
pub struct RenderArea {
    x: u32,
    y: u32, 
    width: u32,
    height: u32,
}

impl Commands {
    pub fn write_buffer<T: Copy>(&mut self, write: BufferWrite<'_, T>) {
        todo!()
    }
    pub fn copy_buffer_to_buffer<R: ops::RangeBounds<usize>>(&mut self, copy: BufferCopy<'_, R>) {
        todo!()
    }
    pub fn begin_render_pass(&mut self, render_pass: RenderPass<'_>) {
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
