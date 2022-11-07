use crate::prelude::*;

use std::borrow::{Borrow, BorrowMut};
use std::default::default;
use std::marker::PhantomData;
use std::mem;
use std::ops;

use ash::vk;

use bitflags::bitflags;

pub struct Present<'a> {
    pub wait_semaphore: &'a BinarySemaphore<'a>,
    pub swapchain: &'a Swapchain<'a>,
}

pub struct Submit<'a> {
    pub wait_semaphore: &'a BinarySemaphore<'a>,
    pub signal_semaphore: &'a BinarySemaphore<'a>,
}

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
    pub(crate) submit: Option<Submit<'a>>,
    pub(crate) present: Option<Present<'a>>,
    pub(crate) debug_name: String,
}

impl<'a> Executor<'a> {
    pub fn add<'b: 'a, const N: usize, F: ops::FnMut(&mut Commands) -> Result<()> + 'a>(
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

    pub fn submit(&mut self, submit: Submit<'a>) {
        self.submit = Some(submit);
    }

    pub fn present(&mut self, present: Present<'a>) {
        self.present = Some(present);
    }

    pub fn complete(self) -> Result<Executable<'a>> {
        let Executor {
            device,
            nodes,
            submit,
            present,
            ..
        } = self;

        let Device {
            logical_device,
            command_pool,
            ..
        } = device;

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool: *command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..default()
        };

        let command_buffer =
            unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }
                .map_err(|_| Error::Creation)?
                .pop()
                .ok_or(Error::Creation)?;

        let fence_create_info = vk::FenceCreateInfo {
            flags: vk::FenceCreateFlags::SIGNALED,
            ..default()
        };

        let fence = unsafe { logical_device.create_fence(&fence_create_info, None) }
            .map_err(|_| Error::Creation)?;

        Ok(Executable {
            device,
            nodes,
            submit,
            present,
            command_buffer,
            fence,
        })
    }
}

pub struct Executable<'a> {
    pub(crate) device: &'a Device<'a>,
    pub(crate) nodes: Vec<Node<'a>>,
    pub(crate) submit: Option<Submit<'a>>,
    pub(crate) present: Option<Present<'a>>,
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) fence: vk::Fence,
}

impl ops::FnMut<()> for Executable<'_> {
    extern "rust-call" fn call_mut(&mut self, args: ()) {
        let Executable {
            device,
            nodes,
            submit,
            present,
            command_buffer,
            fence,
        } = self;

        let Device {
            logical_device,
            queue_family_indices,
            ..
        } = device;

        let queue_family_index = queue_family_indices[0];

        let queue = unsafe { logical_device.get_device_queue(queue_family_index as _, 0) };

        unsafe {
            logical_device.wait_for_fences(&[*fence], true, u64::MAX);
        }

        unsafe {
            logical_device.reset_fences(&[*fence]);
        }

        unsafe {
            logical_device.begin_command_buffer(*command_buffer, &default());
        }

        for node in nodes {
            let qualifiers = node
                .resources
                .iter()
                .map(|resource| resource.resolve())
                .collect::<Vec<_>>();

            let mut commands = Commands {
                device: &device,
                qualifiers: &qualifiers,
                command_buffer: &command_buffer,
            };

            (node.task)(&mut commands);
        }

        unsafe {
            logical_device.end_command_buffer(*command_buffer);
        }

        if let Some(submit) = submit {
            let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

            let submit_info = {
                let p_wait_dst_stage_mask = wait_dst_stage_mask.as_ptr();

                let wait_semaphore_count = 1;

                let p_wait_semaphores = &submit.wait_semaphore.semaphore;

                let signal_semaphore_count = 1;

                let p_signal_semaphores = &submit.signal_semaphore.semaphore;

                let command_buffer_count = 1;

                let p_command_buffers = command_buffer;

                vk::SubmitInfo {
                    p_wait_dst_stage_mask,
                    wait_semaphore_count,
                    p_wait_semaphores,
                    signal_semaphore_count,
                    p_signal_semaphores,
                    command_buffer_count,
                    p_command_buffers,
                    ..default()
                }
            };

            unsafe {
                logical_device.queue_submit(queue, &[submit_info], *fence);
            }
        }

        if let Some(present) = present {
            let present_info = {
                let swapchain_count = 1;

                let p_swapchains = &present.swapchain.handle;

                let wait_semaphore_count = 1;

                let p_wait_semaphores = &present.wait_semaphore.semaphore;

                let image_index =
                    (*present.swapchain.last_acquisition_index.lock().unwrap()).unwrap();

                let p_image_indices = &image_index;

                vk::PresentInfoKHR {
                    wait_semaphore_count,
                    p_wait_semaphores,
                    swapchain_count,
                    p_swapchains,
                    p_image_indices,
                    ..default()
                }
            };

            unsafe {
                present.swapchain.loader.queue_present(queue, &present_info);
            }
        }
    }
}

impl ops::FnOnce<()> for Executable<'_> {
    type Output = ();

    extern "rust-call" fn call_once(mut self, args: ()) {
        self.call_mut(())
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

impl Resource<'_> {
    pub(crate) fn resolve(self) -> Qualifier {
        match self {
            Resource::Buffer(call, access) => Qualifier::Buffer((call)(), access),
            Resource::Image(call, access) => Qualifier::Image((call)(), access),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Qualifier {
    Buffer(Buffer, BufferAccess),
    Image(Image, ImageAccess),
}

pub struct Task<'a, const N: usize, F: ops::FnMut(&mut Commands) -> Result<()>> {
    pub resources: [Resource<'a>; N],
    pub task: F,
}

pub struct Node<'a> {
    pub resources: Vec<Resource<'a>>,
    pub task: Box<dyn ops::FnMut(&mut Commands) -> Result<()> + 'a>,
}

pub fn non_optimizer(graph: &mut Executor<'_>) {}
