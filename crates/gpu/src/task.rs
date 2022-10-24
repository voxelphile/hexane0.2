use crate::image::Image;
use crate::commands::Commands;
use crate::buffer::Buffer;

use std::ops;
use std::mem;
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;

use bitflags::bitflags;

pub enum Dependency {
    Implicit,
    Explicit(usize, usize),
}

pub struct Graph<'a> {
    optimizer: &'a dyn ops::Fn(&mut Graph<'a>),
    executor: Option<Executor>,
    tasks: Vec<Task<'a>>,
    dependencies: Vec<Dependency>,
}

impl<'a> Graph<'a> {
    pub fn new(optimizer: &'a dyn ops::Fn(&mut Graph<'a>)) -> Self {
        Self {
            optimizer,
            executor: None,
            tasks: vec![],
            dependencies: vec![],
        }
    }

    pub fn add(&mut self, task: Task<'a>) {
        let _ = self.executor.take();

        self.tasks.push(task);

        (self.optimizer)(self);
    }

    pub fn execute(&mut self) {
        if let None = self.executor { 
            self.executor = Some(Executor::new(&self));
        }

        self.executor.as_mut().unwrap().execute();
    }
}

pub struct Executor;

impl Executor {
    fn new(graph: &'_ Graph<'_>) -> Self {
        todo!()
    }

    fn execute(&mut self) {
        todo!()
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

pub fn non_optimizer(graph: &mut Graph<'_>) { }
