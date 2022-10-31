use crate::buffer::Buffer;
use crate::image::Image;
use crate::pipeline::Pipeline;

use std::default::default;
use std::ops;

pub struct Commands {}

pub struct BufferWrite<'a, T: Copy> {
    pub buffer: Buffer,
    pub offset: usize,
    pub source: &'a [T],
}

pub struct BufferCopy<R: ops::RangeBounds<usize>> {
    pub from: Buffer,
    pub to: Buffer,
    pub range: R,
}

pub struct DrawIndexed {
    pub index_count: usize,
}

pub struct RenderPass<'a> {
    pub color: &'a [Attachment],
    pub depth: Option<Attachment>,
    pub render_area: RenderArea,
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
    pub image: Image,
    pub load_op: LoadOp,
    pub clear: Clear,
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
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Commands {
    pub fn write_buffer<T: Copy>(&mut self, write: BufferWrite<T>) {
        todo!()
    }
    pub fn copy_buffer_to_buffer<R: ops::RangeBounds<usize>>(&mut self, copy: BufferCopy<R>) {
        todo!()
    }
    pub fn begin_render_pass(&mut self, render_pass: RenderPass<'_>) {
        todo!()
    }
    pub fn end_render_pass(&mut self) {
        todo!()
    }
    pub fn set_pipeline(&mut self, pipeline: &Pipeline) {
        todo!()
    }
    pub fn draw_indexed(&mut self, draw: DrawIndexed) {
        todo!()
    }
}
