#![feature(try_blocks)]

use gpu::prelude::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

fn root_path() -> Option<PathBuf> {
    let current_dir = env::current_dir().ok()?;

    let valid_parents = ["/target/debug", "/target/release", "/bin"];

    for valid_parent in valid_parents {
        if current_dir.ends_with(valid_parent) {}

        let root_dir = try {
            let current_dir = current_dir;

            for i in 0..valid_parents.split("/").len() {
                current_dir = current_dir.parent()?;
            }

            current_dir
        };

        if root_dir.is_some() {
            return root_dir;
        }
    }

    return current_dir;
}

fn main() {
    println!("Hello, world!");

    let root_path = root_path();

    let source_path = source_path.join("source");
    let asset_path = root_path.join("assets");

    if root_paths.len() == 0 {
        panic!("must have at least one root path");
    }

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
        source_path,
        asset_path,
        ..default()
    });

    use ShaderType::*;

    let vertex = Shader(Vertex, "triangle", &["common"]);

    let fragment = Shader(Fragment, "triangle", &["common"]);

    let pipeline = pipeline_compiler.create_pipeline(PipelineInfo::Graphics {
        shaders: &[vertex, fragment],
        attachments: &[Attachment {
            format: swapchain.format(),
            ..default()
        }],
        ..default()
    });

    loop {}
}
