use crate::image::Image;
use crate::commands::Commands;
use crate::buffer::Buffer;

use std::ops;
use std::mem;
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;

use bitflags::bitflags;

pub struct Executor<'a> {
    optimizer: &'a dyn ops::Fn(&mut Executor<'a>),
    tasks: Vec<Task<'a>>,
}

impl<'a> Executor<'a> {
    pub fn new(optimizer: &'a dyn ops::Fn(&mut Executor<'a>)) -> Self {
        Self {
            optimizer,
            tasks: vec![],
        }
    }

    pub fn add(&mut self, task: Task<'a>) {
        self.tasks.push(task);

        (self.optimizer)(self);
    }

    pub fn execute(&mut self) {
    }
}

#[derive(Clone, Copy)]
pub enum ImageAccess {
    None,
    ShaderReadOnly,
    VertexShaderReadOnly, 
    FragmentShaderReadOnly,
    ComputeShaderReadOnly,
    ShaderWriteOnly,
    VertexShaderWriteOnly, 
    FragmentShaderWriteOnly,
    ComputeShaderWriteOnly,
    ShaderReadWrite,
    VertexShaderReadWrite, 
    FragmentShaderReadWrite,
    ComputeShaderReadWrite,
    TransferRead,
    TransferWrite,
    ColorAttachment,
    DepthAttachment,
    StencilAttachment,
    DepthStencilAttachment,
    DepthAttachmentReadOnly,
    StencilAttachmentReadOnly,
    DepthStencilAttachmentReadOnly,
    ResolveWrite,
    Present
}

#[derive(Clone, Copy)]
pub enum BufferAccess {
    None,
    ShaderReadOnly,
    VertexShaderReadOnly, 
    FragmentShaderReadOnly,
    ComputeShaderReadOnly,
    ShaderWriteOnly,
    VertexShaderWriteOnly, 
    FragmentShaderWriteOnly,
    ComputeShaderWriteOnly,
    ShaderReadWrite,
    VertexShaderReadWrite, 
    FragmentShaderReadWrite,
    ComputeShaderReadWrite,
    TransferRead,
    TransferWrite,
    HostTransferRead,
    HostTransferWrite,
}

#[derive(Clone, Copy)]
pub enum Resource {
    Buffer(Buffer, BufferAccess),
    Image(Image, ImageAccess),
}

pub struct Task<'a> 
{
    pub resources: Vec<Resource>,
    pub task: &'a dyn ops::FnMut(&'a mut Commands),
}

pub fn non_optimizer(graph: &mut Executor<'_>) { }
