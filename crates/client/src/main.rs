#![feature(try_blocks)]
#![feature(box_syntax)]
#![feature(default_free_fn)]

mod camera;

use crate::camera::Camera;
use common::bits::Bitset;
use common::octree::SparseOctree;
use common::transform::{Region, Transform, Transformation};
use common::voxel::Voxel;

use gpu::prelude::*;
use math::prelude::*;

use std::cell::Cell;
use std::default::default;
use std::env;
use std::mem;
use std::ops;
use std::path;
use std::time;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
}; 

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

const SMALL_SIZE: usize = 512;
const REALLY_LARGE_SIZE: usize = 1_000_000_000;
const SUPER_LARGE_SIZE: usize = 4_000_000_000;

const WORLD_SIZE: usize = 64;

pub type Vertex = (f32, f32, f32);
pub type Color = [f32; 4];
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

    tracy_client::Client::start();
    profiling::register_thread!("main");

    let mut event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Hexane | FPS 0")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 1920,
            height: 1080,
        })
        .build(&event_loop)
        .unwrap();

    let mut resolution = window.inner_size().into();
    
    let (width, height) = resolution;

    use common::mesh::*;
    use common::octree::*;
    use common::voxel::*;
    use math::prelude::*;

    use noise::{NoiseFn, Perlin, Seedable};

    use rand::Rng;

    let mut rng = rand::thread_rng();

    let perlin = Perlin::new(420);

    let mut octree = SparseOctree::<Voxel>::new();

    for x in 0..WORLD_SIZE {
        for z in 0..WORLD_SIZE {
            const BASE_HEIGHT: usize = 64;
            const VARIATION_HEIGHT: usize = 10;

            let mut height = BASE_HEIGHT as isize;

            const OCTAVES: usize = 4;
            const SAMPLE_BASIS: f64 = 128.0;

            for i in 1..=OCTAVES {
                let sample_x = x as f64 / (SAMPLE_BASIS / i as f64);
                let sample_z = z as f64 / (SAMPLE_BASIS / i as f64);

                let diff = (perlin.get([sample_x, 0.0, sample_z])
                    * (VARIATION_HEIGHT as f64 / i as f64)) as isize;

                height += diff;
            }

            let height = height as usize;

            for y in 0..height {
                if y == height - 1 {
                    octree.place(Vector::new([x, y, z]), Voxel { id: Id::Grass });
                } else {
                    octree.place(Vector::new([x, y, z]), Voxel { id: Id::Dirt });
                }
            }
        }
        if x % 100 == 0 {
            octree.optimize();
        }
    }
    octree.optimize();


    println!("Finished world generation.");

    let mesh: Mesh = octree.transform(Transformation {
        regions: [Region {
            start: Vector::new([0, 0, 0]),
            end: Vector::new([WORLD_SIZE, 128, WORLD_SIZE]),
        }],
        lod: 1,
    });

    println!("Finished mesh generation.");

    let bitset: Bitset = octree.transform(Transformation {
        regions: [Region {
            start: Vector::new([0, 0, 0]),
            end: Vector::new([WORLD_SIZE, 128, WORLD_SIZE]),
        }],
        lod: 1,
    });

    dbg!(bitset.data().len() * mem::size_of::<u32>());

    println!("Finished bitset generation.");

    let root_path = root_path().expect("failed to get root path");

    let source_path = root_path.join("source");
    let asset_path = root_path.join("assets");

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
        asset_path: &asset_path,
        ..default()
    });

    use ShaderType::*;

    let draw_pipeline = pipeline_compiler
        .create_graphics_pipeline(GraphicsPipelineInfo {
            shaders: [Shader(Vertex, "voxel", &[]), Shader(Fragment, "voxel", &[])],
            color: [gpu::prelude::Color {
                format: device.presentation_format(swapchain.get()).unwrap(),
                ..default()
            }],
            depth: Some(default()),
            raster: Raster {
                face_cull: FaceCull::BACK,
                ..default()
            },
            ..default()
        })
        .expect("failed to create pipeline");

    let physics_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "physics", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let mut light_depth_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(width as _, height as _),
                usage: ImageUsage::DEPTH_STENCIL,
                format: Format::D32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut depth_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(width as _, height as _),
                usage: ImageUsage::DEPTH_STENCIL,
                format: Format::D32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let staging_buffer = device
        .create_buffer(BufferInfo {
            size: SUPER_LARGE_SIZE,
            memory: Memory::HOST_ACCESS,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let vertex_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let transform_buffer = device
        .create_buffer(BufferInfo {
            size: SMALL_SIZE,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");
    
    let bitset_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
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

    let light_camera_buffer = device
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

    let vertices = mesh.vertices();

    let colors = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ];

    let mut camera = Cell::new(Camera::Perspective {
        fov: 90.0 * std::f32::consts::PI / 360.0,
        clip: (0.001, 500.0),
        aspect_ratio: width as f32 / height as f32,
        position: Vector::new([WORLD_SIZE as f32 / 2.0, 100.0, (WORLD_SIZE as f32) / 2.0]),
        rotation: default(),
    });

    let mut light_camera = Cell::new(Camera::Orthographic {
        left: -50.0,
        right: 50.0,
        top: 50.0,
        bottom: -50.0,
        clip: (0.1, 1000.0),
        position: Vector::new([WORLD_SIZE as f32 / 2.0, 100.0, (WORLD_SIZE as f32) / 2.0]),
        rotation: default(),
    });

    let camera_info = Cell::new(default());
                
    camera_info.set(CameraInfo {
                    projection: camera.get().projection(),
                });

    let light_camera_info = Cell::new(default());

    let info = Cell::new(default());

    let updated = Cell::new(true);

    let vertex_buffer = || vertex_buffer;
    let transform_buffer = || transform_buffer;
    let bitset_buffer = || bitset_buffer;
    let info_buffer = || info_buffer;
    let camera_buffer = || camera_buffer;
    let staging_buffer = || staging_buffer;
    let depth_image = || depth_img.get();
    let light_camera_buffer = || light_camera_buffer;
    let light_depth_image = || light_depth_img.get();
    let present_image = || {
        device
            .acquire_next_image(Acquire {
                swapchain: swapchain.get(),
                semaphore: Some(&acquire_semaphore),
            })
            .unwrap()
    };

    let sens = 1.0;
    let speed = 10.0;

    let mut cursor_captured = false;

    let mut draw_executable: Option<Executable<'_>> = None;

    let startup_instant = time::Instant::now();
    let mut last_instant = startup_instant;

    profiling::finish_frame!();

    event_loop.run_return(|event, _, control_flow| {
        profiling::scope!("event loop", "ev");
        *control_flow = ControlFlow::Poll;

        let current_instant = time::Instant::now();

        let delta_time = current_instant.duration_since(last_instant).as_secs_f64();

        last_instant = current_instant;

        info.set(Info {
            time: current_instant
                .duration_since(startup_instant)
                .as_secs_f32(),
            delta_time: delta_time as f32,
            ..info.get()
        });

        if cursor_captured {
               /* let mut camera = camera.get();

                let mut position = camera.get_position();

                let pitch = camera.pitch();
                let roll = camera.roll();

                let mut movement = Vector::default();

                let mut diff = Matrix::identity();

                let mut direction = Vector::<f32, 3>::default();

                direction[0] += entity_input.right as u8 as f32;
                direction[0] -= entity_input.left as u8 as f32;
                direction[1] += entity_input.up as u8 as f32;
                direction[1] -= entity_input.down as u8 as f32;
                direction[2] += entity_input.forward as u8 as f32;
                direction[2] -= entity_input.backward as u8 as f32;

                diff[3][0] = direction[0];
                diff[3][2] = direction[2];

                movement[1] = direction[1];

                let diff = diff * roll * pitch;

                let mut oriented_direction = Vector::<f32, 4>::new(*diff[3]);

                oriented_direction[1] = 0.0;
                oriented_direction[3] = 0.0;

                oriented_direction = if oriented_direction.magnitude() > 0.0 {
                    oriented_direction.normalize()
                } else {
                    oriented_direction
                };

                movement[0] = oriented_direction[0];
                movement[2] = oriented_direction[2];
        
                movement *= speed;

                info.set(Info {
                    input:EntityInput {
                        movement,
                        ..info.get().input
                    },
                    ..info.get()
                });*/
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                profiling::scope!("draw executable", "ev");

                if let Some(e) = &draw_executable {
                    window.set_title(&format!("Hexane | FPS {}", e.fps()));
                }

                if !cursor_captured {
                        info.set(Info {
                            entity_input: default(),
                            ..info.get()
                        });
                        dbg!("yo");
                }

                (draw_executable.as_mut().unwrap())();

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

                    let x_rot = (y_diff * delta_time) / sens;
                    let y_rot = (x_diff * delta_time) / sens;

                    info.set(Info {
                    entity_input: EntityInput {
                        look: Vector::new([x_diff as _, y_diff as _, 0.0, 0.0]),
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

                match button {
                    Left => {
                        cursor_captured = true;
                        window.set_cursor_icon(winit::window::CursorIcon::Crosshair);
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                            .expect("could not grab mouse cursor");
                    }
                    _ => {}
                }
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
                    W => entity_input.forward = (input.state == winit::event::ElementState::Pressed) as _,
                    A => entity_input.left = (input.state == winit::event::ElementState::Pressed) as _,
                    S => entity_input.backward = (input.state == winit::event::ElementState::Pressed) as _,
                    D => entity_input.right = (input.state == winit::event::ElementState::Pressed) as _,
                    Space => entity_input.up = (input.state == winit::event::ElementState::Pressed) as _,
                    LShift => entity_input.down = (input.state == winit::event::ElementState::Pressed) as _,
                    R => {
                        pipeline_compiler.refresh_graphics_pipeline(&draw_pipeline).unwrap();
                        pipeline_compiler.refresh_compute_pipeline(&physics_pipeline).unwrap();
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

                resolution = (width, height);

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
    
                camera_info.set(CameraInfo {
                    projection: camera.get().projection(),
                });

                depth_img.set(
                    device
                        .create_image(ImageInfo {
                            extent: ImageExtent::TwoDim(width as _, height as _),
                            usage: ImageUsage::DEPTH_STENCIL,
                            format: Format::D32Sfloat,
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

                let mut draw_executor = device
                    .create_executor(ExecutorInfo {
                        swapchain: swapchain.get(),
                        ..default()
                    })
                    .expect("failed to create executor");

                record_update(Update {
                    executor: &mut draw_executor,
                    vertices: &vertices,
                    octree: &octree,
                    bitset: &bitset,
                    colors: &colors,
                    camera_info: &camera_info,
                    light_camera_info: &light_camera_info,
                    info: &info,
                    updated: &updated,
                    staging_buffer: &staging_buffer,
                    vertex_buffer: &vertex_buffer,
                    transform_buffer: &transform_buffer,
                    bitset_buffer: &bitset_buffer,
                    info_buffer: &info_buffer,
                    camera_buffer: &camera_buffer,
                    light_camera_buffer: &light_camera_buffer,
                });

                record_physics(Physics {
                    executor: &mut draw_executor,
                    pipeline: &physics_pipeline,
                    info_buffer: &info_buffer,
                    camera_buffer: &camera_buffer,
                    transform_buffer: &transform_buffer,
                    bitset_buffer: &bitset_buffer,
                });

                record_draw(Draw {
                    executor: &mut draw_executor,
                    vertices: &vertices,
                    pipeline: &draw_pipeline,
                    present_image: &present_image,
                    depth_image: &depth_image,
                    vertex_buffer: &vertex_buffer,
                    transform_buffer: &transform_buffer,
                    bitset_buffer: &bitset_buffer,
                    resolution,
                    info_buffer: &info_buffer,
                    camera_buffer: &camera_buffer,
                });

                draw_executor.submit(Submit {
                    wait_semaphore: &acquire_semaphore,
                    signal_semaphore: &present_semaphore,
                });

                draw_executor.present(Present {
                    wait_semaphore: &present_semaphore,
                });

                draw_executable = Some(
                    draw_executor
                        .complete()
                        .expect("failed to complete executor"),
                );

                updated.set(true);
            }
            _ => (),
        }
    });
}

pub const VERTEX_OFFSET: usize = 2048;
pub const INDEX_OFFSET: usize = 1_000_000_000;

struct Update<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub vertices: &'b [common::mesh::Vertex],
    pub octree: &'b SparseOctree<Voxel>,
    pub bitset: &'b Bitset,
    pub colors: &'b [Color],
    pub camera_info: &'b Cell<CameraInfo>,
    pub light_camera_info: &'b Cell<CameraInfo>,
    pub info: &'b Cell<Info>,
    pub updated: &'b Cell<bool>,
    pub vertex_buffer: &'b dyn ops::Fn() -> Buffer,
    pub transform_buffer: &'b dyn ops::Fn() -> Buffer,
    pub bitset_buffer: &'b dyn ops::Fn() -> Buffer,
    pub staging_buffer: &'b dyn ops::Fn() -> Buffer,
    pub info_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
    pub light_camera_buffer: &'b dyn ops::Fn() -> Buffer,
}

struct Draw<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub resolution: (u32, u32),
    pub present_image: &'b dyn ops::Fn() -> Image,
    pub depth_image: &'b dyn ops::Fn() -> Image,
    pub vertex_buffer: &'b dyn ops::Fn() -> Buffer,
    pub transform_buffer: &'b dyn ops::Fn() -> Buffer,
    pub bitset_buffer: &'b dyn ops::Fn() -> Buffer,
    pub info_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
    pub pipeline: &'b Pipeline<'b, 2, 1>,
    pub vertices: &'b [common::mesh::Vertex],
}

struct Physics<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
    pub transform_buffer: &'b dyn ops::Fn() -> Buffer,
    pub bitset_buffer: &'b dyn ops::Fn() -> Buffer,
    pub info_buffer: &'b dyn ops::Fn() -> Buffer,
    pub pipeline: &'b Pipeline<'b, 1, 0>,
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
struct CameraInfo {
    projection: Matrix<f32, 4, 4>,
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
    look: Vector<f32, 4>,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Push {
    pub info_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub bitset_buffer: Buffer,
}

#[profiling::function]
fn record_physics<'a, 'b: 'a>(physics: Physics<'a, 'b>) {
    use Resource::*;

    let Physics {
        executor,
        camera_buffer,
        info_buffer,
        bitset_buffer,
        transform_buffer,
        pipeline,
    } = physics;

    executor.add(Task {
        resources: [
            Buffer(camera_buffer, BufferAccess::ShaderReadOnly),
            Buffer(info_buffer, BufferAccess::ShaderReadOnly),
            Buffer(bitset_buffer, BufferAccess::ShaderReadOnly),
        ],
        task: move |commands| {
            commands.set_pipeline(&pipeline)?;
            
            commands.push_constant(PushConstant {
                data: Push {
                    info_buffer: (info_buffer)(),
                    camera_buffer: (camera_buffer)(),
                    vertex_buffer: 0.into(),
                    transform_buffer: (transform_buffer)(),
                    bitset_buffer: (bitset_buffer)(),
                },
                pipeline: &pipeline,
            })?;

            commands.dispatch(1, 1, 1)?;

            commands.pipeline_barrier(PipelineBarrier {
                src_stage: PipelineStage::ComputeShader,
                dst_stage: PipelineStage::VertexShader,
                barriers: &[Barrier::Buffer {
                    buffer: 0,
                    offset: 0,
                    size: SMALL_SIZE,
                    src_access: Access::SHADER_WRITE,
                    dst_access: Access::SHADER_READ,
                }],
            })
        },
    });
}

#[profiling::function]
fn record_draw<'a, 'b: 'a>(draw: Draw<'a, 'b>) {
    use Resource::*;

    let Draw {
        executor,
        present_image,
        depth_image,
        vertex_buffer,
        transform_buffer,
        bitset_buffer,
        info_buffer,
        camera_buffer,
        resolution,
        vertices,
        pipeline,
    } = draw;

    executor.add(Task {
        resources: [
            Image(present_image, ImageAccess::ColorAttachment),
            Image(depth_image, ImageAccess::DepthStencilAttachment),
            Buffer(info_buffer, BufferAccess::ShaderReadOnly),
            Buffer(camera_buffer, BufferAccess::ShaderReadOnly),
            Buffer(vertex_buffer, BufferAccess::ShaderReadOnly),
            Buffer(transform_buffer, BufferAccess::ShaderReadOnly),
            Buffer(bitset_buffer, BufferAccess::ShaderReadOnly),
        ],
        task: move |commands| {
            let (width, height) = resolution;

            commands.pipeline_barrier(PipelineBarrier {
                src_stage: PipelineStage::TopOfPipe,
                dst_stage: PipelineStage::ColorAttachmentOutput,
                barriers: &[Barrier::Image {
                    image: 0,
                    old_layout: ImageLayout::Undefined,
                    new_layout: ImageLayout::ColorAttachmentOptimal,
                    src_access: Access::empty(),
                    dst_access: Access::COLOR_ATTACHMENT_WRITE,
                }],
            })?;

            commands.pipeline_barrier(PipelineBarrier {
                src_stage: PipelineStage::TopOfPipe,
                dst_stage: PipelineStage::EarlyFragmentTests,
                barriers: &[Barrier::Image {
                    image: 1,
                    old_layout: ImageLayout::Undefined,
                    new_layout: ImageLayout::DepthAttachmentOptimal,
                    src_access: Access::empty(),
                    dst_access: Access::DEPTH_ATTACHMENT_READ | Access::DEPTH_ATTACHMENT_WRITE,
                }],
            })?;

            commands.set_resolution(resolution)?;

            commands.start_rendering(Render {
                color: &[Attachment {
                    image: 0,
                    load_op: LoadOp::Clear,
                    clear: Clear::Color(0.1, 0.4, 0.8, 1.0),
                }],
                depth: Some(Attachment {
                    image: 1,
                    load_op: LoadOp::Clear,
                    clear: Clear::Depth(1.0),
                }),
                render_area: RenderArea {
                    width,
                    height,
                    ..default()
                },
            })?;

            commands.set_pipeline(&pipeline)?;

            commands.push_constant(PushConstant {
                data: Push {
                    info_buffer: (info_buffer)(),
                    camera_buffer: (camera_buffer)(),
                    vertex_buffer: (vertex_buffer)(),
                    transform_buffer: (transform_buffer)(),
                    bitset_buffer: (bitset_buffer)(),
                },
                pipeline: &pipeline,
            })?;

            const VERTICES_PER_CUBE: usize = 6;

            commands.draw(gpu::prelude::Draw {
                vertex_count: vertices.len() * VERTICES_PER_CUBE,
            })?;

            commands.end_rendering()?;

            commands.pipeline_barrier(PipelineBarrier {
                src_stage: PipelineStage::ColorAttachmentOutput,
                dst_stage: PipelineStage::BottomOfPipe,
                barriers: &[Barrier::Image {
                    image: 0,
                    old_layout: ImageLayout::ColorAttachmentOptimal,
                    new_layout: ImageLayout::Present,
                    src_access: Access::COLOR_ATTACHMENT_WRITE,
                    dst_access: Access::empty(),
                }],
            })
        },
    });
}

#[profiling::function]
fn record_update<'a, 'b: 'a>(update: Update<'a, 'b>) {
    use Resource::*;

    let Update {
        executor,
        staging_buffer,
        vertex_buffer,
        transform_buffer,
        bitset_buffer,
        info_buffer,
        camera_buffer,
        light_camera_buffer,
        vertices,
        octree,
        bitset,
        colors,
        updated,
        camera_info,
        light_camera_info,
        info,
    } = update;

    executor.add(Task {
        resources: [
            Buffer(staging_buffer, BufferAccess::TransferRead),
            Buffer(camera_buffer, BufferAccess::TransferWrite),
            Buffer(info_buffer, BufferAccess::TransferWrite),
            Buffer(vertex_buffer, BufferAccess::TransferWrite),
            Buffer(transform_buffer, BufferAccess::TransferWrite),
            Buffer(light_camera_buffer, BufferAccess::TransferWrite),
            Buffer(bitset_buffer, BufferAccess::TransferWrite),
        ],
        task: move |commands| {

            commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: 512,
                src: &[light_camera_info.get()],
            })?;

            commands.copy_buffer_to_buffer(BufferCopy {
                from: 0,
                to: 5,
                src: 512,
                dst: 0,
                size: mem::size_of::<CameraInfo>(),
            })?;

            commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: 1024,
                src: &[info.get()],
            })?;

            commands.copy_buffer_to_buffer(BufferCopy {
                from: 0,
                to: 2,
                src: 1024,
                dst: 0,
                size: mem::size_of::<Info>(),
            })?;

            if updated.get() {
                updated.set(false);
            
                commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: 0,
                src: &[camera_info.get()],
            })?;

            commands.copy_buffer_to_buffer(BufferCopy {
                from: 0,
                to: 1,
                src: 0,
                dst: 0,
                size: mem::size_of::<CameraInfo>(),
            })?;

                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: VERTEX_OFFSET,
                    src: &[vertices.len()],
                })?;

                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: VERTEX_OFFSET + mem::size_of::<u32>(),
                    src: vertices,
                })?;

                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: INDEX_OFFSET,
                    src: &[octree.size() as u32],
                })?;
/*
                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: INDEX_OFFSET + mem::size_of::<u32>(),
                    src: &[octree.nodes().len() as u32],
                })?;

                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: INDEX_OFFSET + 2 * mem::size_of::<u32>(),
                    src: &octree.nodes(),
                })?;
  */              
                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: INDEX_OFFSET,
                    src: &[bitset.len() as u32],
                })?;

                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset:  INDEX_OFFSET + mem::size_of::<u32>(),
                    src: &bitset.data(),
                })?;

                commands.copy_buffer_to_buffer(BufferCopy {
                    from: 0,
                    to: 3,
                    src: VERTEX_OFFSET,
                    dst: 0,
                    size: REALLY_LARGE_SIZE,
                })?;

                commands.copy_buffer_to_buffer(BufferCopy {
                    from: 0,
                    to: 6,
                    src: INDEX_OFFSET,
                    dst: 0,
                    size: REALLY_LARGE_SIZE,
                })?;
            }
            Ok(())
        },
    });
}
