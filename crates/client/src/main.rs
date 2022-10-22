#![feature(try_blocks)]
#![feature(default_free_fn)]

use gpu::prelude::*;

use std::default::default;
use std::env; 
use std::path;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

const REALLY_LARGE_SIZE: usize = 1_000_000;

fn root_path() -> Option<path::PathBuf> {
    let current_dir = env::current_dir().ok()?;

    let valid_parents = ["/target/debug", "/target/release", "/bin"];

    for valid_parent in valid_parents {
        if !current_dir.ends_with(valid_parent) {
            continue;
        }

        let root_dir: Option<path::PathBuf> = try {
            let cursor = current_dir;

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

    let (width, height) = window.inner_size().into();

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

    let swapchain = device
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

    let pipeline = pipeline_compiler.create_graphics_pipeline(GraphicsPipelineInfo {
        shaders: &[vertex, fragment],
        color: &[Attachment {
            format: Format::Undefined,
            //format: swapchain.format(),
            ..default()
        }],
        ..default()
    });
/*
    let staging_buffer = device.create_buffer(BufferInfo {
        size: REALLY_LARGE_SIZE,
        memory: Memory::HOST_ACCESS_RANDOM,
        debug_name: "Staging Buffer",
        ..default()
    });

    let general_buffer = device.create_buffer(BufferInfo {
        size: REALLY_LARGE_SIZE,
        debug_name: "General Buffer",
        ..default()
    });

    let acquire_semaphore = device.create_binary_semaphore(BinarySemaphoreInfo {
        debug_name: "Acquire Semaphore",
        ..default(),
    });
    
    let present_semaphore = device.create_binary_semaphore(BinarySemaphoreInfo {
        debug_name: "Present Semaphore",
        ..default(),
    });
*/
    loop {}
}

fn record_task_list(data: &mut GpuData) -> Graph<Task> {
    //takes a type which implements the trait Optimizer
    //NonOptimizer does nothing to the graph
    let GpuData { 
        general_buffer,
        staging_buffer,
        ..
    } = &mut data;

    let task_graph = Graph::new(non_optimizer);

    use Access::*;

    task_graph.add(task! {
        |cmd, &mut staging_buffer| {
            //write to staging buffer
        }
    });

}
