use crate::buffer::Buffer;
use crate::commands::Commands;
use crate::image::Image;

use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::mem;
use std::ops;

use bitflags::bitflags;

pub struct Executor<'a> {
    optimizer: &'a dyn ops::Fn(&mut Executor<'a>),
    nodes: Vec<Node<'a>>,
}

impl<'a> Executor<'a> {
    pub fn new(optimizer: &'a dyn ops::Fn(&mut Executor<'a>)) -> Self {
        Self {
            optimizer,
            nodes: vec![],
        }
    }

    pub fn add<'b, F: ops::FnMut(&mut Commands) + 'a>(&mut self, task: Task<'b, F>) {
        let Task { task, resources } = task;

        self.nodes.push(Node {
            resources: resources.iter().cloned().collect(),
            task: box task,
        });

        (self.optimizer)(self);
    }

    pub fn execute(&mut self) {}
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
    Present,
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

pub struct Task<'a, F: ops::FnMut(&mut Commands)> {
    pub resources: &'a [Resource],
    pub task: F,
}

pub struct Node<'a> {
    pub resources: Vec<Resource>,
    pub task: Box<dyn ops::FnMut(&mut Commands) + 'a>,
}

pub fn non_optimizer(graph: &mut Executor<'_>) {}
