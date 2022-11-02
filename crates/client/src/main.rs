#![feature(try_blocks)]
#![feature(default_free_fn)]

use gpu::prelude::*;

use std::default::default;
use std::env;
use std::mem;
use std::ops;
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

    let mut device = context
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

    let mut pipeline_compiler = device.create_pipeline_compiler(PipelineCompilerInfo {
        //default language for shader compiler is glsl
        #[cfg(debug_assertions)]
        compiler: ShaderCompiler::glslc(default()),
        source_path: &source_path,
        asset_path: &asset_path,
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
            memory: Memory::HOST_ACCESS,
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

    let acquire_semaphore = device
        .create_binary_semaphore(BinarySemaphoreInfo {
            debug_name: "Acquire Semaphore",
            ..default()
        })
        .expect("failed to create semaphore");

    let present_semaphore = device
        .create_binary_semaphore(BinarySemaphoreInfo {
            debug_name: "Present Semaphore",
            ..default()
        })
        .expect("failed to create semaphore");

    let vertices = [(0.0, -0.5, 0.0), (0.5, 0.5, 0.0), (-0.5, 0.5, 0.0)];
    let indices = [0, 1, 2];

    let mut executor = device.create_executor(default());

    let general_buffer = || general_buffer;
    let staging_buffer = || staging_buffer;
    let present_image = || swapchain.acquire();

    record_update(Update {
        executor: &mut executor,
        vertices: &vertices,
        indices: &indices,
        general_buffer: &general_buffer,
        staging_buffer: &staging_buffer,
    });

    record_draw(Draw {
        executor: &mut executor,
        indices: &indices,
        pipeline: &pipeline,
        present_image: &present_image,
        resolution,
    });

    executor.submit(Submit {});

    executor.present(Present {});

    let executable = executor.complete();

    loop {
        (executable)();
    }
}

pub const VERTEX_OFFSET: usize = 0;
pub const INDEX_OFFSET: usize = 1024;

struct Update<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub vertices: &'b [Vertex],
    pub indices: &'b [Index],
    pub general_buffer: &'b dyn ops::Fn() -> Buffer,
    pub staging_buffer: &'b dyn ops::Fn() -> Buffer,
}

struct Draw<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub resolution: (u32, u32),
    pub present_image: &'b dyn ops::Fn() -> Image,
    pub pipeline: &'b Pipeline<'b>,
    pub indices: &'b [Index],
}

fn record_draw<'a, 'b: 'a>(draw: Draw<'a, 'b>) {
    use Resource::*;

    let Draw {
        executor,
        present_image,
        resolution,
        indices,
        pipeline,
    } = draw;

    executor.add(Task {
        resources: [Image(present_image, ImageAccess::ColorAttachment)],
        task: move |commands| {
            let (width, height) = resolution;

            commands.begin_render_pass(RenderPass {
                color: &[Attachment {
                    image: 0,
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

            commands.set_pipeline(&pipeline);

            commands.draw_indexed(DrawIndexed {
                index_count: indices.len(),
            });

            commands.end_render_pass();
        },
    });
}

fn record_update<'a, 'b: 'a>(update: Update<'a, 'b>) {
    use Resource::*;

    let Update {
        executor,
        staging_buffer,
        general_buffer,
        vertices,
        indices,
    } = update;

    executor.add(Task {
        resources: [Buffer(staging_buffer, BufferAccess::HostTransferWrite)],
        task: move |commands| {
            commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: 0,
                source: vertices,
            });

            commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: INDEX_OFFSET,
                source: indices,
            });
        },
    });

    executor.add(Task {
        resources: [
            Buffer(staging_buffer, BufferAccess::TransferRead),
            Buffer(general_buffer, BufferAccess::TransferWrite),
        ],
        task: move |commands| {
            commands.copy_buffer_to_buffer(BufferCopy {
                from: 0,
                to: 1,
                range: 0..REALLY_LARGE_SIZE,
            })
        },
    });
}
