use crate::prelude::*;

use std::borrow::{Borrow, BorrowMut};
use std::default::default;
use std::marker::PhantomData;
use std::mem;
use std::ops;
use std::slice;

use ash::vk;

use bitflags::bitflags;

pub struct Present<'a> {
    pub wait_semaphore: &'a BinarySemaphore<'a>,
    pub swapchain: Swapchain,
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
            resources,
            ..
        } = device;

        let resources = resources.lock().unwrap();

        if let Some(present) = &present {
            resources
                .swapchains
                .get(present.swapchain)
                .ok_or(Error::ResourceNotFound)?;
        }

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
            resources,
            staging_address_buffer,
            staging_address_memory,
            general_address_buffer,
            descriptor_set,
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

        {
            let resources = resources.lock().unwrap();

            let mut addresses = [0u64; DESCRIPTOR_COUNT as usize];

            let mut descriptor_buffer_infos = vec![];

            for i in 0..DESCRIPTOR_COUNT as usize {
                if let Some(internal_buffer) = resources.buffers.get((i as u32).into()) {
                    let buffer_device_address_info = vk::BufferDeviceAddressInfo {
                        buffer: internal_buffer.buffer,
                        ..default()
                    };

                    addresses[i] = unsafe {
                        logical_device.get_buffer_device_address(&buffer_device_address_info)
                    };

                    descriptor_buffer_infos.push(vk::DescriptorBufferInfo {
                        buffer: internal_buffer.buffer,
                        offset: 0,
                        range: internal_buffer.size as _,
                    })
                }
            }

            drop(resources);

            let address_buffer_size = (DESCRIPTOR_COUNT * mem::size_of::<u64>() as u32) as u64;

            let dst = unsafe {
                logical_device.map_memory(
                    *staging_address_memory,
                    0,
                    address_buffer_size as _,
                    vk::MemoryMapFlags::empty(),
                )
            }
            .unwrap();

            unsafe { slice::from_raw_parts_mut(dst as *mut _, addresses.len()) }
                .copy_from_slice(&addresses[..]);

            unsafe {
                logical_device.unmap_memory(*staging_address_memory);
            }

            let regions = [vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: address_buffer_size as _,
            }];

            unsafe {
                logical_device.cmd_copy_buffer(
                    *command_buffer,
                    *staging_address_buffer,
                    *general_address_buffer,
                    &regions,
                );
            }

            let descriptor_buffer_info = vk::DescriptorBufferInfo {
                buffer: *general_address_buffer,
                offset: 0,
                range: address_buffer_size as _,
            };

            let write_descriptor_set1 = {
                let p_buffer_info = &descriptor_buffer_info;

                vk::WriteDescriptorSet {
                    dst_set: *descriptor_set,
                    dst_binding: 4, //MAGIC NUMBER SEE context.rs
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    p_buffer_info,
                    ..default()
                }
            };

            let write_descriptor_set2 = {
                let p_buffer_info = descriptor_buffer_infos.as_ptr();

                vk::WriteDescriptorSet {
                    dst_set: *descriptor_set,
                    dst_binding: 3, //MAGIC NUMBER SEE context.rs
                    descriptor_count: descriptor_buffer_infos.len() as _,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    p_buffer_info,
                    ..default()
                }
            };

            unsafe {
                logical_device
                    .update_descriptor_sets(&[write_descriptor_set1, write_descriptor_set2], &[]);
            }
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

            (node.task)(&mut commands).unwrap();
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
            let resources = resources.lock().unwrap();

            let internal_swapchain = resources.swapchains.get(present.swapchain).unwrap();

            let present_info = {
                let swapchain_count = 1;

                let p_swapchains = &internal_swapchain.handle;

                let wait_semaphore_count = 1;

                let p_wait_semaphores = &present.wait_semaphore.semaphore;

                let image_index = internal_swapchain.last_acquisition_index.unwrap();

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
                internal_swapchain
                    .loader
                    .queue_present(queue, &present_info);
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
