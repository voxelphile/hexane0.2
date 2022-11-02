use crate::prelude::*;

use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::mem;
use std::ops;

use bitflags::bitflags;

pub struct Present {}

pub struct Submit {}

pub struct ExecutorInfo<'a> {
    pub optimizer: &'a dyn ops::Fn(&mut Executor<'a>),
    pub debug_name: &'a str,
}

impl Default for ExecutorInfo<'_> {
    fn default() -> Self {
        Self {
            optimizer: &non_optimizer,
            debug_name: "Executor",
        }
    }
}

pub struct Executor<'a> {
    pub(crate) device: &'a Device<'a>,
    pub(crate) optimizer: &'a dyn ops::Fn(&mut Executor<'a>),
    pub(crate) nodes: Vec<Node<'a>>,
    pub(crate) submit: Option<Submit>,
    pub(crate) present: Option<Present>,
    pub(crate) debug_name: String,
}

impl<'a> Executor<'a> {
    pub fn add<'b: 'a, const N: usize, F: ops::FnMut(&mut Commands) + 'a>(
        &mut self,
        task: Task<'b, N, F>,
    ) {
        let Task { task, resources } = task;

        self.nodes.push(Node {
            resources: resources.to_vec(),
            task: box task,
        });

        (self.optimizer)(self);
    }

    pub fn submit(&mut self, submit: Submit) {
        self.submit = Some(submit);
    }

    pub fn present(&mut self, present: Present) {
        self.present = Some(present);
    }

    pub fn complete(mut self) -> Executable<'a> {
        let Executor {
            device,
            nodes,
            submit,
            present,
            ..
        } = self;

        Executable {
            device,
            nodes,
            submit,
            present,
        }
    }
}

pub struct Executable<'a> {
    pub(crate) device: &'a Device<'a>,
    pub(crate) nodes: Vec<Node<'a>>,
    pub(crate) submit: Option<Submit>,
    pub(crate) present: Option<Present>,
}

impl ops::Fn<()> for Executable<'_> {
    extern "rust-call" fn call(&self, args: ()) {}
}

impl ops::FnMut<()> for Executable<'_> {
    extern "rust-call" fn call_mut(&mut self, args: ()) {
        self.call(())
    }
}

impl ops::FnOnce<()> for Executable<'_> {
    type Output = ();

    extern "rust-call" fn call_once(self, args: ()) {
        self.call(())
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
pub enum Resource<'a> {
    Buffer(&'a dyn ops::Fn() -> Buffer, BufferAccess),
    Image(&'a dyn ops::Fn() -> Image, ImageAccess),
}

pub struct Task<'a, const N: usize, F: ops::FnMut(&mut Commands)> {
    pub resources: [Resource<'a>; N],
    pub task: F,
}

pub struct Node<'a> {
    pub resources: Vec<Resource<'a>>,
    pub task: Box<dyn ops::FnMut(&mut Commands) + 'a>,
}

pub fn non_optimizer(graph: &mut Executor<'_>) {}
