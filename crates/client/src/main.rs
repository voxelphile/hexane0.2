#![feature(try_blocks)]
#![feature(default_free_fn)]

use gpu::prelude::*;

use std::default::default;
use std::env;
use std::mem;
use std::path;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

const REALLY_LARGE_SIZE: usize = 1_000_000;

pub type Vertex = (f32, f32, f32);
pub type Index = u32;

fn root_path() -> Option<path::PathBuf> {
    let current_dir = env::current_dir().ok()?;

    let valid_parents = ["/target/debug", "/target/release", "/bin"];

    for valid_parent in valid_parents {
        if !current_dir.ends_with(valid_parent) {
            continue;
        }

        let root_dir: Option<path::PathBuf> = try {
            let mut cursor = current_dir.clone();

            for i in 0..valid_parent.split("/").count() {
                cursor = cursor.parent().map(path::Path::to_path_buf)?;
            }

            cursor
        };

        if root_dir.is_some() {
            return root_dir;
        }
    }

    Some(current_dir)
}

fn main() {
    println!("Hello, world!");

    let root_path = root_path().expect("failed to get root path");

    let source_path = root_path.join("source");
    let asset_path = root_path.join("assets");

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let resolution = window.inner_size().into();

    let (width, height) = resolution;

    let context = Context::new(ContextInfo {
        enable_validation: true,
        application_name: "Hexane",
        engine_name: "Hexane",
        ..default()
    })
    .expect("failed to create context");

    let device = context
        .create_device(DeviceInfo {
            display: window.raw_display_handle(),
            window: window.raw_window_handle(),
            ..default()
        })
        .expect("failed to create device");

    let mut swapchain = device
        .create_swapchain(SwapchainInfo {
            width,
            height,
            ..default()
        })
        .expect("failed to create swapchain");

    let pipeline_compiler = device.create_pipeline_compiler(PipelineCompilerInfo {
        //default language for shader compiler is glsl
        #[cfg(debug_assertions)]
        compiler: ShaderCompiler::glslc(default()),
        source_path: &source_path,
        output_path: &asset_path,
        ..default()
    });

    use ShaderType::*;

    let vertex = Shader(Vertex, "triangle", &["common"]);

    let fragment = Shader(Fragment, "triangle", &["common"]);

    let pipeline = pipeline_compiler
        .create_graphics_pipeline(GraphicsPipelineInfo {
            shaders: &[vertex, fragment],
            color: &[Color {
                format: Format::Undefined,
                //format: swapchain.format(),
                ..default()
            }],
            ..default()
        })
        .expect("failed to create pipeline");

    let staging_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            memory: Memory::HOST_ACCESS_RANDOM,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let general_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let acquire_semaphore = device.create_binary_semaphore(BinarySemaphoreInfo {
        debug_name: "Acquire Semaphore",
        ..default()
    });

    let present_semaphore = device.create_binary_semaphore(BinarySemaphoreInfo {
        debug_name: "Present Semaphore",
        ..default()
    });

    let mut executor = Executor::new(&non_optimizer);

    loop {
        let mut gpu = GpuData {
            resolution,
            vertices: &[(0.0, -0.5, 0.0), (0.5, 0.5, 0.0), (-0.5, 0.5, 0.0)],
            indices: &[0, 1, 2],
            present_image: swapchain.acquire(),
            pipeline,
            general_buffer,
            staging_buffer,
        };

        record_update(&mut executor, gpu);
        record_draw(&mut executor, gpu);

        executor.execute();
    }
}

pub const VERTEX_OFFSET: usize = 0;
pub const INDEX_OFFSET: usize = 1024;

#[derive(Clone, Copy)]
struct GpuData<'a> {
    pub resolution: (u32, u32),
    pub vertices: &'a [Vertex],
    pub indices: &'a [Index],
    pub pipeline: Pipeline,
    pub general_buffer: Buffer,
    pub staging_buffer: Buffer,
    pub present_image: Image,
}

fn record_draw<'a, 'b: 'a>(executor: &mut Executor<'a>, gpu: GpuData<'b>) {
    use Resource::*;

    executor.add(Task {
        resources: &[Image(gpu.present_image, ImageAccess::ColorAttachment)],
        task: move |commands| {
            let (width, height) = gpu.resolution;

            commands.begin_render_pass(RenderPass {
                color: &[Attachment {
                    image: gpu.present_image,
                    load_op: LoadOp::Clear,
                    clear: Clear::Color(0.0, 0.0, 0.0, 1.0),
                }],
                depth: None,
                render_area: RenderArea {
                    width,
                    height,
                    ..default()
                },
            });

            commands.set_pipeline(gpu.pipeline);

            commands.draw_indexed(DrawIndexed {
                index_count: gpu.indices.len(),
            });

            commands.end_render_pass();
        },
    });
}

fn record_update<'a, 'b: 'a>(executor: &mut Executor<'a>, gpu: GpuData<'b>) {
    use Resource::*;

    executor.add(Task {
        resources: &[Buffer(gpu.staging_buffer, BufferAccess::HostTransferWrite)],
        task: move |commands| {
            commands.write_buffer(BufferWrite {
                buffer: gpu.staging_buffer,
                offset: 0,
                source: gpu.vertices,
            });

            commands.write_buffer(BufferWrite {
                buffer: gpu.staging_buffer,
                offset: INDEX_OFFSET,
                source: gpu.indices,
            });
        },
    });

    executor.add(Task {
        resources: &[
            Buffer(gpu.staging_buffer, BufferAccess::TransferRead),
            Buffer(gpu.general_buffer, BufferAccess::TransferWrite),
        ],
        task: move |commands| {
            commands.copy_buffer_to_buffer(BufferCopy {
                from: gpu.staging_buffer,
                to: gpu.general_buffer,
                range: 0..REALLY_LARGE_SIZE,
            })
        },
    });
}
