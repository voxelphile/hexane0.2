#![feature(try_blocks)]
#![feature(box_syntax)]
#![feature(default_free_fn)]

mod camera;

use crate::camera::Camera;
use common::bits::Bitset;
use common::convert::{Conversion, Convert, Region};
use common::octree::SparseOctree;
use common::voxel::Voxel;

use gpu::prelude::*;
use math::prelude::*;

use std::cell::Cell;
use std::cmp;
use std::default::default;
use std::env;
use std::io;
use std::mem;
use std::num;
use std::ops;
use std::path;
use std::sync::{Arc, Mutex};
use std::time;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

const SMALL_SIZE: usize = 512;
const REALLY_LARGE_SIZE: usize = 200_000_000;

const REGION_SIZE: usize = 512;
const CHUNK_SIZE: usize = 64;
const AXIS_MAX_CHUNKS: usize = 4;
const LOD: usize = 3;
const PREPASS_SCALE: usize = 2;
const MAX_BLOCKS: usize = 1024;
const BLOCK_DETAIL: usize = 8;

pub type Vertex = (f32, f32, f32);
pub type Color = [f32; 4];
pub type Index = u32;

fn root_path() -> Option<path::PathBuf> {
    let current_dir = env::current_dir().ok()?;

    let valid_parents = ["\\target\\debug", "\\target\\release", "\\bin"];

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

#[repr(u32)]
#[derive(Debug)]
enum Sound {
    GrassWalk1 = 1,
    GrassWalk2 = 2,
    GrassWalk3 = 3,
    GrassWalk4 = 4,
    GrassWalk5 = 5,
    GrassWalk6 = 6,
    GrassWalk7 = 7,
    GrassWalk8 = 8,
    GrassWalk9 = 9,
    GrassWalk10 = 10,
    DirtWalk1 = 11,
    DirtWalk2 = 12,
    DirtWalk3 = 13,
    DirtWalk4 = 14,
    DirtWalk5 = 15,
    DirtWalk6 = 16,
    DirtWalk7 = 17,
    DirtWalk8 = 18,
    DirtWalk9 = 19,
    DirtWalk10 = 20,
    DirtWalk11 = 21,
    StoneWalk1 = 22,
    StoneWalk2 = 23,
    StoneWalk3 = 24,
    StoneWalk4 = 25,
    StoneWalk5 = 26,
    StoneWalk6 = 27,
    StoneWalk7 = 28,
    StoneWalk8 = 29,
    StoneWalk9 = 30,
    StoneWalk10 = 31,
    StoneWalk11 = 32,
}

impl Sound {
    fn name(&self) -> String {
        use convert_case::{Case, Casing};
        format!("{}.ogg", format!("{:?}", self).to_case(Case::Snake))
    }
}

fn main() {
    println!("Hello, client!");

    //tracy_client::Client::start();
    profiling::register_thread!("main");

    let mut event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Hexane | Frame time: 0 ms")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 1920 / 2,
            height: 1080 / 2,
        })
        .build(&event_loop)
        .unwrap();

    let mut resolution = Cell::new(window.inner_size().into());

    let (width, height) = resolution.get();

    let root_path = root_path().expect("failed to get root path");

    let source_path = root_path.join("source");
    let asset_path = root_path.join("assets");
    let shader_asset_path = asset_path.join("shaders");
    let sound_asset_path = asset_path.join("audio");

    let context = Context::new(ContextInfo {
        enable_validation: false,
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

    let mut swapchain = Cell::new(
        device
            .create_swapchain(SwapchainInfo {
                width,
                height,
                present_mode: PresentMode::DoNotWaitForVBlank,
                ..default()
            })
            .expect("failed to create swapchain"),
    );

    let mut pipeline_compiler = device.create_pipeline_compiler(PipelineCompilerInfo {
        //default language for shader compiler is glsl
        compiler: ShaderCompiler::glslc(default()),
        source_path: &source_path,
        asset_path: &shader_asset_path,
        ..default()
    });

    use ShaderType::*;

    let draw_pipeline = pipeline_compiler
        .create_graphics_pipeline(GraphicsPipelineInfo {
            shaders: [Shader(Vertex, "fx", &[]), Shader(Fragment, "draw", &[])],
            color: [gpu::prelude::Color {
                format: Format::Rgba32Sfloat,
                blend: None,
            }],
            depth: None,
            ..default()
        })
        .expect("failed to create pipeline");
    let resolve_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "resolve", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    let history_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "history", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let upscale_pipeline = pipeline_compiler
        .create_graphics_pipeline(GraphicsPipelineInfo {
            shaders: [Shader(Vertex, "fx", &[]), Shader(Fragment, "upscale", &[])],
            color: [gpu::prelude::Color {
                format: device.presentation_format(swapchain.get()).unwrap(),
                blend: None,
            }],
            depth: None,
            ..default()
        })
        .expect("failed to create pipeline");
    
    let before_luminosity_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "before_luminosity", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let after_luminosity_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "after_luminosity", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let luminosity_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_luminosity", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let input_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "input", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let box_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_box", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let build_struct_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_struct", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let after_build_struct_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "after_build_struct", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let build_blocks_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_blocks", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let build_world_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_world", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let build_bound_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_initial_bound", &[]),
            ..default()
        })
        .expect("failed to create pipeline");
    
    let move_world_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "move_world", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let after_world_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "after_world", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    /*let vertex_pipeline = pipeline_compiler
            .create_compute_pipeline(ComputePipelineInfo {
                shader: Shader(Compute, "build_mesh", &[]),
                ..default()
            })
            .expect("failed to create pipeline");
    */
    let noise_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_noise", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let perlin_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_perlin", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let worley_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_worley", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let physics_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "physics", &[]),
            ..default()
        })
        .expect("failed to create pipeline");


    let mut depth_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::DEPTH,
                format: Format::D32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut prepass_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );
    
    let mut dir_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );
    
    let mut pos_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );
    
    let mut history_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );
    
    let mut history_dir_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );
    let mut history_pos_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );
    
    let mut resolve_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut resolve_dir_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(
                    width as usize / PREPASS_SCALE,
                    height as usize / PREPASS_SCALE,
                ),
                usage: ImageUsage::COLOR,
                format: Format::Rgba32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut noise_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::ThreeDim(16, 16, 16),
                usage: ImageUsage::TRANSFER_DST,
                format: Format::Rgba32Uint,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut perlin_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::ThreeDim(128, 128, 128),
                usage: ImageUsage::TRANSFER_DST,
                format: Format::R32Uint,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut worley_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::ThreeDim(128, 128, 128),
                usage: ImageUsage::TRANSFER_DST,
                format: Format::R32Uint,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut chunk_images = vec![];

    for _ in 0..2 {
        chunk_images.push(
            device
                .create_image(ImageInfo {
                    extent: ImageExtent::ThreeDim(REGION_SIZE, REGION_SIZE, REGION_SIZE),
                    usage: ImageUsage::TRANSFER_DST,
                    format: Format::R16Uint,
                    ..default()
                })
                .expect("failed to create depth image"),
        );
    }
        chunk_images.push(
            device
                .create_image(ImageInfo {
                    extent: ImageExtent::ThreeDim(BLOCK_DETAIL, BLOCK_DETAIL, BLOCK_DETAIL * MAX_BLOCKS),
                    usage: ImageUsage::TRANSFER_DST,
                    format: Format::R16Uint,
                    ..default()
                })
                .expect("failed to create depth image"),
        );
        for lod in 1..=LOD {
            let lod1 = 2i32.pow(lod as _) as usize;
            let lod_size = REGION_SIZE / (lod1);
            chunk_images.push(
                device
                    .create_image(ImageInfo {
                        extent: ImageExtent::ThreeDim(lod_size, lod_size, lod_size),
                        usage: ImageUsage::TRANSFER_DST,
                        format: Format::R16Uint,
                        ..default()
                    })
                    .expect("failed to create depth image"),
            );
        }

    let general_staging_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            memory: Memory::HOST_ACCESS,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");
    
    let bounding_buffer = device
        .create_buffer(BufferInfo {
            size: 1000000,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");
    
    let entity_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");
    
    let luminosity_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");


    let chunk_staging_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            memory: Memory::HOST_ACCESS,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let noise_staging_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            memory: Memory::HOST_ACCESS,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let world_buffer = device
        .create_buffer(BufferInfo {
            size: 1000000,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let mersenne_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let transform_buffer = device
        .create_buffer(BufferInfo {
            size: 100000,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let sound_buffer = device
        .create_buffer(BufferInfo {
            size: 4096,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let sound_staging_buffer = device
        .create_buffer(BufferInfo {
            size: 4096,
            memory: Memory::HOST_ACCESS,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let void_staging_buffer = device
        .create_buffer(BufferInfo {
            size: 4096,
            memory: Memory::HOST_ACCESS,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let rigidbody_buffer = device
        .create_buffer(BufferInfo {
            size: SMALL_SIZE,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let info_buffer = device
        .create_buffer(BufferInfo {
            size: SMALL_SIZE,
            debug_name: "Color Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let camera_buffer = device
        .create_buffer(BufferInfo {
            size: SMALL_SIZE,
            debug_name: "Color Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let input_buffer = device
        .create_buffer(BufferInfo {
            size: SMALL_SIZE,
            debug_name: "Color Buffer",
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

    let colors = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ];

    let mut camera = Cell::new(Camera::Perspective {
        fov: 120.0 * std::f32::consts::PI / 360.0,
        clip: (0.1, 500.0),
        aspect_ratio: width as f32 / height as f32,
        position: Vector::new([16.0, 48.0, 16.0]),
        rotation: default(),
    });

    let camera_info = Cell::new(default());

    camera_info.set(CameraInfo {
        projection: camera.get().projection(),
        inv_projection: camera.get().projection().inverse(),
        far: match camera.get() {
            Camera::Perspective { clip, .. } => clip.1,
            _ => unreachable!(),
        },
        resolution: Vector::new([
            resolution.get().0 as f32 / PREPASS_SCALE as f32,
            resolution.get().1 as f32 / PREPASS_SCALE as f32,
        ]),
    });

    let info = Cell::new(Info::default());

    let basic_update = Cell::new(true);
    let block_update = Cell::new(true);
    let noise_update = Cell::new(true);
    let world_update = Cell::new(true);

    let physics_time_accum = Cell::new(0.0);

    let vertex_count = Cell::new(0);

    let input_buffer = || input_buffer;
    let bounding_buffer = || bounding_buffer;
    let entity_buffer = || entity_buffer;
    let luminosity_buffer = || luminosity_buffer;
    let mersenne_buffer = || mersenne_buffer;
    let transform_buffer = || transform_buffer;
    let sound_buffer = || sound_buffer;
    let sound_staging_buffer = || sound_staging_buffer;
    let void_staging_buffer = || void_staging_buffer;
    let rigidbody_buffer = || rigidbody_buffer;
    let info_buffer = || info_buffer;
    let camera_buffer = || camera_buffer;
    let world_buffer = || world_buffer;
    let general_staging_buffer = || general_staging_buffer;
    let chunk_staging_buffer = || chunk_staging_buffer;
    let noise_staging_buffer = || noise_staging_buffer;
    let depth_image = || depth_img.get();
    let prepass_image = || prepass_img.get();
    let dir_image = || dir_img.get();
    let pos_image = || pos_img.get();
    let history_image = || history_img.get();
    let history_dir_image = || history_dir_img.get();
    let history_pos_image = || history_pos_img.get();
    let resolve_image = || resolve_img.get();
    let resolve_dir_image = || resolve_dir_img.get();
    let noise_image = || noise_img.get();
    let perlin_image = || perlin_img.get();
    let worley_image = || worley_img.get();
    let present_image = || {
        device
            .acquire_next_image(Acquire {
                swapchain: swapchain.get(),
                semaphore: Some(acquire_semaphore),
            })
            .unwrap()
    };

    let sens = 1.0;
    let speed = 10.0;

    let mut cursor_captured = false;

    let mut executable: Option<Executable<'_>> = None;

    let startup_instant = time::Instant::now();
    let mut last_instant = startup_instant;

    profiling::finish_frame!();

    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

    let stream_handle = Arc::new(Mutex::new(stream_handle));

    event_loop.run_return(|event, _, control_flow| {
        let sound_asset_path = sound_asset_path.clone();
        let stream_handle = stream_handle.clone();

        profiling::scope!("event loop", "ev");
        *control_flow = ControlFlow::Poll;

        let current_instant = time::Instant::now();

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                profiling::scope!("draw executable", "ev");

                if let Some(e) = &executable {
                    let mut ms = 1000.0 * (1.0 / e.fps() as f64);

                    if(ms.is_infinite()) {
                        ms = 0.0;
                    }

                    let ms = ms as usize;

                    window.set_title(&format!("Hexane | Frame time: {} ms", ms));
                }

                if !cursor_captured {
                    info.set(Info {
                        entity_input: default(),
                        ..info.get()
                    });
                }

                let delta_time = current_instant.duration_since(last_instant).as_secs_f64();

                last_instant = current_instant;

                info.set(Info {
                    time: current_instant
                        .duration_since(startup_instant)
                        .as_secs_f32(),
                    delta_time: info.get().delta_time + delta_time as f32,
                    ..info.get()
                });

                physics_time_accum.set(physics_time_accum.get() + delta_time as f32);

                (executable.as_mut().unwrap())();

                profiling::finish_frame!();
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                window_id,
            } => {
                profiling::scope!("cursor moved", "ev");
                if cursor_captured {
                    let winit::dpi::PhysicalPosition { x, y } = position;

                    let winit::dpi::PhysicalSize { width, height } = window.inner_size();

                    let x_diff = x - width as f64 / 2.0;
                    let y_diff = y - height as f64 / 2.0;

                    window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                        width as i32 / 2,
                        height as i32 / 2,
                    ));

                    info.set(Info {
                        entity_input: EntityInput {
                            look: info.get().entity_input.look
                                + Vector::new([x_diff as _, y_diff as _, 0.0, 0.0]),
                            ..info.get().entity_input
                        },
                        ..info.get()
                    });
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { button, .. },
                window_id,
            } => {
                profiling::scope!("mouse input", "ev");
                use winit::event::MouseButton::*;

                let mut entity_input = info.get().entity_input;
                
                match button {
                    Left => {
                        cursor_captured = true;
                        window.set_cursor_icon(winit::window::CursorIcon::Crosshair);
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                            .expect("could not grab mouse cursor");
                        entity_input.action1 = true as _;     
                    }
                    Right => {
                        entity_input.action2 = true as _;
                    }
                    _ => {}
                }

                info.set(Info {
                    entity_input,
                    ..info.get()
                });
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                window_id,
            } => {
                profiling::scope!("keyboard input", "ev");
                let Some(key_code) = input.virtual_keycode else {
                    return;
                };

                let mut entity_input = info.get().entity_input;

                use winit::event::VirtualKeyCode::*;

                match key_code {
                    W => {
                        entity_input.forward =
                            (input.state == winit::event::ElementState::Pressed) as _
                    }
                    A => {
                        entity_input.left =
                            (input.state == winit::event::ElementState::Pressed) as _
                    }
                    S => {
                        entity_input.backward =
                            (input.state == winit::event::ElementState::Pressed) as _
                    }
                    D => {
                        entity_input.right =
                            (input.state == winit::event::ElementState::Pressed) as _
                    }
                    Space => {
                        entity_input.up = (input.state == winit::event::ElementState::Pressed) as _
                    }
                    LShift => {
                        entity_input.down =
                            (input.state == winit::event::ElementState::Pressed) as _
                    }
                    R => {
                        pipeline_compiler
                            .refresh_compute_pipeline(&input_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&resolve_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_graphics_pipeline(&draw_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&noise_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&perlin_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&build_world_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&physics_pipeline)
                            .unwrap();
                    }
                    Escape => {
                        cursor_captured = false;
                        window.set_cursor_icon(winit::window::CursorIcon::Default);
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::None)
                            .expect("could not grab mouse cursor");
                    }
                    _ => {}
                };
                info.set(Info {
                    entity_input,
                    ..info.get()
                });
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id,
            } => {
                profiling::scope!("resized", "ev");

                let winit::dpi::PhysicalSize { width, height } = size;

                if((width, height) == resolution.get()) {
                    return;
                }   

                swapchain.set(
                    device
                        .create_swapchain(SwapchainInfo {
                            width,
                            height,
                            old_swapchain: Some(swapchain.get()),
                            present_mode: PresentMode::DoNotWaitForVBlank,
                            ..default()
                        })
                        .expect("failed to create swapchain"),
                );
                
                resolution.set((width, height));

                depth_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::DEPTH,
                            format: Format::D32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );

                prepass_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );

                dir_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );
                
                history_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );
                
                history_dir_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );
                pos_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );
                
                resolve_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );
                
                resolve_dir_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(
                                width as usize / PREPASS_SCALE,
                                height as usize / PREPASS_SCALE,
                            ),
                            usage: ImageUsage::COLOR,
                            format: Format::Rgba32Sfloat,
                            ..default()
                        })
                        .expect("failed to create depth image"),
                );

                let new_camera = {
                    let mut camera = camera.get();

                    let new_aspect_ratio = width as f32 / height as f32;

                    match &mut camera {
                        Camera::Perspective { aspect_ratio, .. } => {
                            *aspect_ratio = new_aspect_ratio
                        }
                        _ => {}
                    }

                    camera
                };

                camera.set(new_camera);

                camera_info.set(CameraInfo {
                    projection: camera.get().projection(),
                    inv_projection: camera.get().projection().inverse(),
                    far: match camera.get() {
                        Camera::Perspective { clip, .. } => clip.1,
                        _ => unreachable!(),
                    },
                    resolution: Vector::new([
                        width as f32 / PREPASS_SCALE as f32,
                        height as f32 / PREPASS_SCALE as f32,
                    ]),
                });

                basic_update.set(true);

                let mut executor = device
                    .create_executor(ExecutorInfo {
                        swapchain: swapchain.get(),
                        ..default()
                    })
                    .expect("failed to create executor");

                use Resource::*;

                executor.add(Task {
                    resources: [Buffer(
                        &general_staging_buffer,
                        BufferAccess::HostTransferWrite,
                    )],
                    task: |commands| {
                        commands.write_buffer(BufferWrite {
                            buffer: 0,
                            offset: 1024,
                            src: &[info.get()],
                        });

                        info.set(Info {
                            entity_input: EntityInput {
                                look: default(),
                                action1: false as _,
                                action2: false as _,
                                ..info.get().entity_input
                            },
                            delta_time: default(),
                            ..info.get()
                        });

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&general_staging_buffer, BufferAccess::TransferRead),
                        Buffer(&info_buffer, BufferAccess::TransferWrite),
                    ],
                    task: |commands| {
                        commands.copy_buffer_to_buffer(BufferCopy {
                            from: 0,
                            to: 1,
                            src: 1024,
                            dst: 0,
                            size: mem::size_of::<Info>(),
                        })?;
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [Buffer(
                        &general_staging_buffer,
                        BufferAccess::HostTransferWrite,
                    )],
                    task: |commands| {
                        if basic_update.get() {
                            commands.write_buffer(BufferWrite {
                                buffer: 0,
                                offset: 8192,
                                src: &[camera_info.get()],
                            })?;
                        }

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&general_staging_buffer, BufferAccess::TransferRead),
                        Buffer(&camera_buffer, BufferAccess::TransferWrite),
                    ],
                    task: |commands| {
                        if basic_update.get() {
                            commands.copy_buffer_to_buffer(BufferCopy {
                                from: 0,
                                to: 1,
                                src: 8192,
                                dst: 0,
                                size: mem::size_of::<CameraInfo>(),
                            })?;
                        }
                        basic_update.set(false);
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [Buffer(
                        &chunk_staging_buffer,
                        BufferAccess::HostTransferWrite,
                    )],
                    task: |commands| {
                        if world_update.get() {
                            commands.write_buffer(BufferWrite {
                                buffer: 0,
                                offset: 0,
                                src: &chunk_images,
                            })?;
                        }

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&chunk_staging_buffer, BufferAccess::TransferRead),
                        Buffer(&world_buffer, BufferAccess::TransferWrite),
                    ],
                    task: |commands| {
                        if world_update.get() {
                            commands.copy_buffer_to_buffer(BufferCopy {
                                from: 0,
                                to: 1,
                                src: 0,
                                dst: 0,
                                size: chunk_images.len() * mem::size_of::<gpu::prelude::Image>(),
                            })?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [Buffer(
                        &noise_staging_buffer,
                        BufferAccess::HostTransferWrite,
                    )],
                    task: |commands| {
                        if noise_update.get() {
                            const W: u32 = 32;
                            const N: u32 = 642;

                            const F: u32 = 1812433253;

                            let index = N;
                            let mut mt = vec![];

                            mt.push(42069);

                            for i in 1..N as usize {
                                let num = num::Wrapping(F)
                                    * num::Wrapping(
                                        (mt[i - 1] ^ (mt[i - 1] >> (W - 2))) + i as u32,
                                    );
                                mt.push(num.0);
                            }
                            commands.write_buffer(BufferWrite {
                                buffer: 0,
                                offset: 0,
                                src: &[index],
                            })?;

                            commands.write_buffer(BufferWrite {
                                buffer: 0,
                                offset: mem::size_of::<u32>(),
                                src: &mt,
                            })?;
                        }

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&noise_staging_buffer, BufferAccess::TransferRead),
                        Buffer(&mersenne_buffer, BufferAccess::TransferWrite),
                    ],
                    task: |commands| {
                        if noise_update.get() {
                            commands.copy_buffer_to_buffer(BufferCopy {
                                from: 0,
                                to: 1,
                                src: 0,
                                dst: 0,
                                size: 100000,
                            })?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&mersenne_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&noise_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if noise_update.get() {
                            commands.set_pipeline(&noise_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildNoisePush {
                                    mersenne_buffer: (mersenne_buffer)(),
                                    noise_image: (noise_image)(),
                                },
                                pipeline: &noise_pipeline,
                            })?;

                            let size = 128;

                            const WORK_GROUP_SIZE: usize = 8;

                            let dispatch_size =
                                (16 as f64 / WORK_GROUP_SIZE as f64).ceil() as usize;
                        
                            commands.dispatch(dispatch_size, dispatch_size, dispatch_size)?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Image(&noise_image, ImageAccess::ComputeShaderReadWrite),
                        Image(&worley_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if noise_update.get() {
                            commands.set_pipeline(&worley_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildPerlinPush {
                                    noise_image: (noise_image)(),
                                    perlin_image: (worley_image)(),
                                },
                                pipeline: &worley_pipeline,
                            })?;

                            const WORK_GROUP_SIZE: usize = 8;

                            let dispatch_size =
                                (128 as f64 / WORK_GROUP_SIZE as f64).ceil() as usize;

                            commands.dispatch(dispatch_size, dispatch_size, dispatch_size)?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Image(&noise_image, ImageAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if noise_update.get() {
                            commands.set_pipeline(&perlin_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildPerlinPush {
                                    noise_image: (noise_image)(),
                                    perlin_image: (perlin_image)(),
                                },
                                pipeline: &perlin_pipeline,
                            })?;

                            const WORK_GROUP_SIZE: usize = 8;

                            let dispatch_size =
                                (128 as f64 / WORK_GROUP_SIZE as f64).ceil() as usize;

                            commands.dispatch(dispatch_size, dispatch_size, dispatch_size)?;
                        }
                        noise_update.set(false);
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&void_staging_buffer, BufferAccess::TransferRead),
                        Buffer(&sound_buffer, BufferAccess::TransferWrite),
                    ],
                    task: |commands| {
                        commands.copy_buffer_to_buffer(BufferCopy {
                            from: 0,
                            to: 1,
                            src: 0,
                            dst: 0,
                            size: 4096,
                        })?;
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&info_buffer, BufferAccess::ComputeShaderReadOnly),
                        Buffer(&input_buffer, BufferAccess::ComputeShaderReadOnly),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&rigidbody_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&sound_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&mersenne_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&luminosity_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&input_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: InputPush {
                                info_buffer: (info_buffer)(),
                                transform_buffer: (transform_buffer)(),
                                rigidbody_buffer: (rigidbody_buffer)(),
                                input_buffer: (input_buffer)(),
                                sound_buffer: (sound_buffer)(),
                                mersenne_buffer: (mersenne_buffer)(),
                                world_buffer: (world_buffer)(),
                                camera_buffer: (camera_buffer)(),
                                luminosity_buffer: (luminosity_buffer)(),
                            },
                            pipeline: &input_pipeline,
                        })?;

                        commands.dispatch(1, 1, 1)
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                            commands.set_pipeline(&move_world_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildWorldPush {
                                    worley_image: (worley_image)(),
                                    perlin_image: (perlin_image)(),
                                    transform_buffer: (transform_buffer)(),
                                    world_buffer: (world_buffer)(),
                                    mersenne_buffer: (mersenne_buffer)(),
                                },
                                pipeline: &move_world_pipeline,
                            })?;

                            const WORK_GROUP_SIZE: usize = 8;

                            let size = REGION_SIZE / WORK_GROUP_SIZE;

                            commands.dispatch(size, size, size)?;
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                        Image(&worley_image, ImageAccess::ComputeShaderReadWrite),
                        Buffer(&mersenne_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if(block_update.get()){
                            commands.set_pipeline(&build_blocks_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildWorldPush {
                                    perlin_image: (perlin_image)(),
                                    worley_image: (worley_image)(),
                                    transform_buffer: (transform_buffer)(),
                                    world_buffer: (world_buffer)(),
                                    mersenne_buffer: (mersenne_buffer)(),
                                },
                                pipeline: &build_blocks_pipeline,
                            })?;

                            commands.dispatch(1, 1, 1)?;
                    }
                        Ok(())
                    },
                });
                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                        Image(&worley_image, ImageAccess::ComputeShaderReadWrite),
                        Buffer(&mersenne_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                            commands.set_pipeline(&build_world_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildWorldPush {
                                    perlin_image: (perlin_image)(),
                                    worley_image: (worley_image)(),
                                    transform_buffer: (transform_buffer)(),
                                    world_buffer: (world_buffer)(),
                                    mersenne_buffer: (mersenne_buffer)(),
                                },
                                pipeline: &build_world_pipeline,
                            })?;

                            const WORK_GROUP_SIZE: usize = 8;

                            let size = REGION_SIZE / WORK_GROUP_SIZE;

                            commands.dispatch(size, size, size)?;
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&box_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: BuildWorldPush {
                                perlin_image: (perlin_image)(),
                                transform_buffer: (transform_buffer)(),
                                worley_image: (worley_image)(),
                                mersenne_buffer: (mersenne_buffer)(),
                                world_buffer: (world_buffer)(),
                            },
                            pipeline: &box_pipeline,
                        })?;

                        const WORK_GROUP_SIZE: usize = 1;

                        let size = (AXIS_MAX_CHUNKS) / WORK_GROUP_SIZE;

                        commands.dispatch(size, size, size)?;
                        world_update.set(false);
                        Ok(())
                    },
                });
                
                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&bounding_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if(block_update.get()) {
                        commands.set_pipeline(&build_bound_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: BuildBoundingPush {
                                world_buffer: (world_buffer)(),
                                bounding_buffer: (bounding_buffer)(),
                            },
                            pipeline: &build_bound_pipeline,
                        })?;

                        commands.dispatch(MAX_BLOCKS, 1, 1)?;
                        }
                        block_update.set(false);
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&after_world_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: BuildWorldPush {
                                perlin_image: (perlin_image)(),
                                transform_buffer: (transform_buffer)(),
                                worley_image: (worley_image)(),
                                world_buffer: (world_buffer)(),
                                    mersenne_buffer: (mersenne_buffer)(),
                            },
                            pipeline: &after_world_pipeline,
                        })?;

                        commands.dispatch(1, 1, 1)?;
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&build_struct_pipeline)?;
                        
                        for lod in 1..=LOD {
                            let lod1 = 2i32.pow(lod as _) as usize;
                            commands.push_constant(PushConstant {
                                data: BuildStructPush {
                                    world_buffer: (world_buffer)(),
                                    lod: lod1 as _,
                                },
                                pipeline: &build_struct_pipeline,
                            })?;
                            
                            const WORK_GROUP_SIZE: usize = 8;
                        
                            let size = REGION_SIZE / (lod1)  / WORK_GROUP_SIZE;

                            commands.dispatch(size, size, size)?;
                        }
                        Ok(())
                    },
                });
                
                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&after_build_struct_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: BuildWorldPush {
                                perlin_image: (perlin_image)(),
                                transform_buffer: (transform_buffer)(),
                                worley_image: (worley_image)(),
                                world_buffer: (world_buffer)(),
                                    mersenne_buffer: (mersenne_buffer)(),
                            },
                            pipeline: &after_build_struct_pipeline,
                        })?;

                        commands.dispatch(1, 1, 1)?;
                        Ok(())
                    },
                });

                const PHYSICS_FIXED_TIME: f32 = 0.01;
                const PHYSICS_TIME_PRECISION: f32 = 1_000_000.0;

                executor.add(Task {
                    resources: [
                        Buffer(&info_buffer, BufferAccess::ComputeShaderReadOnly),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&rigidbody_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&bounding_buffer, BufferAccess::ComputeShaderReadOnly),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&physics_pipeline)?;

                        let mut new_physics_time_accum = physics_time_accum.get();

                        while new_physics_time_accum >= PHYSICS_FIXED_TIME {
                            commands.push_constant(PushConstant {
                                data: PhysicsPush {
                                    fixed_time: PHYSICS_FIXED_TIME,
                                    info_buffer: (info_buffer)(),
                                    transform_buffer: (transform_buffer)(),
                                    rigidbody_buffer: (rigidbody_buffer)(),
                                    world_buffer: (world_buffer)(),
                                    bounding_buffer: (bounding_buffer)(),
                                },
                                pipeline: &physics_pipeline,
                            })?;

                            commands.dispatch(1, 1, 1)?;

                            new_physics_time_accum -= PHYSICS_FIXED_TIME;
                        }

                        physics_time_accum.set(new_physics_time_accum);

                        Ok(())
                    },
                });
                
               executor.add(Task {
                    resources: [
                        Image(&prepass_image, ImageAccess::ColorAttachment),
                        Buffer(&camera_buffer, BufferAccess::ShaderReadOnly),
                        Buffer(&info_buffer, BufferAccess::ShaderReadOnly),
                        Buffer(&transform_buffer, BufferAccess::ShaderReadOnly),
                        Buffer(&world_buffer, BufferAccess::ShaderReadOnly),
                        Buffer(&mersenne_buffer, BufferAccess::ShaderReadWrite),
                        Buffer(&luminosity_buffer, BufferAccess::ShaderReadOnly),
                        Buffer(&entity_buffer, BufferAccess::ShaderReadWrite),
                    ],
                    task: |commands| {
                        let (width, height) = resolution.get();

                        let resolution = (width / PREPASS_SCALE as u32, height / PREPASS_SCALE as u32);

                        commands.set_resolution(resolution)?;

                        commands.set_pipeline(&draw_pipeline)?;

                        commands.start_rendering(Render {
                            color: [Attachment {
                                image: 0,
                                load_op: LoadOp::Clear,
                                clear: Clear::Color(0.0, 0.0, 0.0, 0.0),
                            }],
                            depth: None,
                            render_area: RenderArea {
                                width: width / PREPASS_SCALE as u32,
                                height: height / PREPASS_SCALE as u32,
                                ..default()
                            },
                        })?;

                        commands.push_constant(PushConstant {
                            data: RtxPush {
                                info_buffer: (info_buffer)(),
                                camera_buffer: (camera_buffer)(),
                                transform_buffer: (transform_buffer)(),
                                world_buffer: (world_buffer)(),
                                mersenne_buffer: (mersenne_buffer)(),
                                dir_image: (dir_image)(),
                                pos_image: (pos_image)(),
                                luminosity_buffer: (luminosity_buffer)(),
                                entity_buffer: (entity_buffer)(),
                            },
                            pipeline: &draw_pipeline,
                        })?;


                        commands.draw(gpu::prelude::Draw { vertex_count: 3 })?;

                        commands.end_rendering();
                        Ok(())
                    },
                });
               
               executor.add(Task {
                    resources: [
                        Image(&prepass_image, ImageAccess::ComputeShaderReadWrite),
                        Image(&resolve_image, ImageAccess::ComputeShaderReadWrite),
                        Buffer(&rigidbody_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&resolve_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: ResolvePush {
                                prepass_image: (prepass_image)(),
                                history_image: (history_image)(),
                                resolve_image: (resolve_image)(),
                                dir_image: (dir_image)(),
                                pos_image: (pos_image)(),
                                history_dir_image: (history_dir_image)(),
                                history_pos_image: (history_pos_image)(),
                                rigidbody_buffer: (rigidbody_buffer)(),
                                info_buffer: (info_buffer)(),
                                camera_buffer: (camera_buffer)(),
                                transform_buffer: (transform_buffer)(),
                                world_buffer: (world_buffer)(),
                            },
                            pipeline: &resolve_pipeline,
                        })?;

                        const WORK_GROUP_SIZE: usize = 16;

                        let group_x = (resolution.get().0 as f32 / PREPASS_SCALE as f32 / WORK_GROUP_SIZE as f32).ceil() as usize;
                        let group_y = (resolution.get().1 as f32 / PREPASS_SCALE as f32 / WORK_GROUP_SIZE as f32).ceil() as usize;

                        commands.dispatch(group_x, group_y, 1)?;

                        Ok(())
                    },
                });
               
               executor.add(Task {
                    resources: [
                        Image(&resolve_image, ImageAccess::ComputeShaderReadWrite),
                        Image(&history_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&history_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: HistoryPush {
                                resolve_image: (resolve_image)(),
                                history_image: (history_image)(),
                            },
                            pipeline: &history_pipeline,
                        })?;

                        const WORK_GROUP_SIZE: usize = 16;

                        let group_x = (resolution.get().0 as f32 / PREPASS_SCALE as f32 / WORK_GROUP_SIZE as f32).ceil() as usize;
                        let group_y = (resolution.get().1 as f32 / PREPASS_SCALE as f32 / WORK_GROUP_SIZE as f32).ceil() as usize;

                        commands.dispatch(group_x, group_y, 1)?;

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&luminosity_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&prepass_image, ImageAccess::ComputeShaderReadWrite),
                        Buffer(&info_buffer, BufferAccess::ComputeShaderReadOnly),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&before_luminosity_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: LuminosityPush {
                                luminosity_buffer: (luminosity_buffer)(),
                                prepass_image: (resolve_image)(),
                                info_buffer: (info_buffer)(),
                            },
                            pipeline: &before_luminosity_pipeline,
                        })?;

                        commands.dispatch(1, 1, 1)?;

                        Ok(())
                    },
                });
                
                executor.add(Task {
                    resources: [
                        Buffer(&luminosity_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&prepass_image, ImageAccess::ComputeShaderReadWrite),
                        Buffer(&info_buffer, BufferAccess::ComputeShaderReadOnly),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&luminosity_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: LuminosityPush {
                                luminosity_buffer: (luminosity_buffer)(),
                                prepass_image: (resolve_image)(),
                                info_buffer: (info_buffer)(),
                            },
                            pipeline: &luminosity_pipeline,
                        })?;

                        const WORK_GROUP_SIZE: usize = 16;

                        let group_x = (resolution.get().0 as f32 / PREPASS_SCALE as f32 / WORK_GROUP_SIZE as f32).ceil() as usize;
                        let group_y = (resolution.get().1 as f32 / PREPASS_SCALE as f32 / WORK_GROUP_SIZE as f32).ceil() as usize;

                        commands.dispatch(group_x, group_y, 1)?;

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&luminosity_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&prepass_image, ImageAccess::ComputeShaderReadWrite),
                        Buffer(&info_buffer, BufferAccess::ComputeShaderReadOnly),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&after_luminosity_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: LuminosityPush {
                                luminosity_buffer: (luminosity_buffer)(),
                                prepass_image: (resolve_image)(),
                                info_buffer: (info_buffer)(),
                            },
                            pipeline: &after_luminosity_pipeline,
                        })?;

                        commands.dispatch(1, 1, 1)?;

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Image(&present_image, ImageAccess::ColorAttachment),
                        Image(&prepass_image, ImageAccess::FragmentShaderReadOnly),
                        Buffer(&luminosity_buffer, BufferAccess::ComputeShaderReadOnly),
                    ],
                    task: |commands| {
                        let (width, height) = resolution.get();

                        commands.set_resolution(resolution.get())?;

                        commands.set_pipeline(&upscale_pipeline)?;

                        commands.start_rendering(Render {
                            color: [Attachment {
                                image: 0,
                                load_op: LoadOp::Clear,
                                clear: Clear::Color(0.0, 0.0, 0.0, 0.0),
                            }],
                            depth: None,
                            render_area: RenderArea {
                                width: width,
                                height: height,
                                ..default()
                            },
                        })?;

                        commands.push_constant(PushConstant {
                            data: UpscalePush {
                                from_image: (resolve_image)(),
                                scale: PREPASS_SCALE as u32,
                                luminosity_buffer: (luminosity_buffer)(),
                            },
                            pipeline: &upscale_pipeline,
                        })?;

                        commands.draw(gpu::prelude::Draw { vertex_count: 3 })?;

                        commands.end_rendering()
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&sound_buffer, BufferAccess::TransferRead),
                        Buffer(&sound_staging_buffer, BufferAccess::TransferWrite),
                    ],
                    task: |commands| {
                        commands.copy_buffer_to_buffer(BufferCopy {
                            from: 0,
                            to: 1,
                            src: 0,
                            dst: 0,
                            size: 4096,
                        })?;
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [Buffer(
                        &sound_staging_buffer,
                        BufferAccess::HostTransferRead,
                    )],
                    task: move |commands| {
                        let len = commands
                            .read_buffer::<u32>(BufferRead {
                                buffer: 0,
                                offset: 0,
                            })
                            .unwrap();

                        let sounds = commands
                            .read_buffer::<[u32; 100]>(BufferRead {
                                buffer: 0,
                                offset: mem::size_of::<u32>(),
                            })
                            .unwrap();

                        for i in 0..len as usize {
                            let sound = unsafe { mem::transmute::<_, Sound>(sounds[i]) };

                            let mut sound_path = sound_asset_path.clone();
                            sound_path.push(sound.name());

                            let file = std::fs::File::open(sound_path).unwrap();

                            let stream_handle = stream_handle.clone();

                            let stream_handle_guard = stream_handle.lock().unwrap();

                            let play = stream_handle_guard.play_once(file).unwrap();
                            play.set_volume(1.0);
                            play.detach();
                        }

                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [Image(&present_image, ImageAccess::Present)],
                    task: |commands| {
                        commands.submit(Submit {
                            wait_semaphore: acquire_semaphore,
                            signal_semaphore: present_semaphore,
                        })?;

                        commands.present(Present {
                            wait_semaphore: present_semaphore,
                        })
                    },
                });

                executable = Some(executor.complete().expect("failed to complete executor"));
            }
            _ => (),
        }
    });
}

pub const VERTEX_OFFSET: usize = 2048;
pub const INDEX_OFFSET: usize = 1_000_000_000;

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
struct CameraInfo {
    projection: Matrix<f32, 4, 4>,
    inv_projection: Matrix<f32, 4, 4>,
    far: f32,
    resolution: Vector<f32, 2>,
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
struct Info {
    time: f32,
    delta_time: f32,
    entity_input: EntityInput,
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
struct EntityInput {
    up: u32,
    down: u32,
    left: u32,
    right: u32,
    forward: u32,
    backward: u32,
    action1: u32,
    action2: u32,
    look: Vector<f32, 4>,
}


#[derive(Clone, Copy)]
#[repr(C)]
pub struct LuminosityPush {
    pub luminosity_buffer: Buffer,
    pub info_buffer: Buffer,
    pub prepass_image: Image,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RtxPush {
    pub info_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub world_buffer: Buffer,
    pub mersenne_buffer: Buffer,
    pub dir_image: Image,
    pub pos_image: Image,
    pub luminosity_buffer: Buffer,
    pub entity_buffer: Buffer,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ResolvePush {
    pub prepass_image: Image,
    pub history_image: Image,
    pub resolve_image: Image,
    pub dir_image: Image,
    pub pos_image: Image,
    pub history_dir_image: Image,
    pub history_pos_image: Image,
    pub rigidbody_buffer: Buffer,
    pub info_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub world_buffer: Buffer,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HistoryPush {
    pub resolve_image: Image,
    pub history_image: Image,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct UpscalePush {
    pub from_image: Image,
    pub scale: u32,
    pub luminosity_buffer: Buffer,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InputPush {
    pub info_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub rigidbody_buffer: Buffer,
    pub input_buffer: Buffer,
    pub sound_buffer: Buffer,
    pub mersenne_buffer: Buffer,
    pub world_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub luminosity_buffer: Buffer,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PhysicsPush {
    pub fixed_time: f32,
    pub info_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub rigidbody_buffer: Buffer,
    pub world_buffer: Buffer,
    pub bounding_buffer: Buffer,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildBitsetPush {
    pub world_buffer: Buffer,
    pub bitset_buffer: Buffer,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildWorldPush {
    pub world_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub perlin_image: Image,
    pub worley_image: Image,
    pub mersenne_buffer: Buffer,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildBoundingPush {
    pub world_buffer: Buffer,
    pub bounding_buffer: Buffer,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildStructPush {
    pub world_buffer: Buffer,
    pub lod: u32,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildMeshPush {
    pub world_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub perlin_image: Image,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildNoisePush {
    pub mersenne_buffer: Buffer,
    pub noise_image: Image,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BuildPerlinPush {
    pub noise_image: Image,
    pub perlin_image: Image,
}
