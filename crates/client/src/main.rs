#![feature(try_blocks)]
#![feature(box_syntax)]
#![feature(default_free_fn)]

mod camera;

use crate::camera::Camera;

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

const REALLY_LARGE_SIZE: usize = 1_000_000_000;

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

#[profiling::function]
fn main() {
    println!("Hello, world!");

    profiling::register_thread!("main")

    use common::mesh::*;
    use common::octree::*;
    use common::voxel::*;
    use math::prelude::*;

    let mut octree = SparseOctree::<Voxel>::new();

    use noise::{NoiseFn, Perlin, Seedable};

    let perlin = Perlin::new(1);
    let val = perlin.get([42.4, 37.7, 2.8]);

    for x in 0..64 {
        for z in 0..64 {
            let height = ((perlin.get([x as f64 / 64.0, 0.0, z as f64 / 64.0]) * 10.0) as isize
                + 20) as usize;
            for y in 0..height {
                octree.place(Vector::new([x, y, z]), Voxel { id: Id::Dirt });
            }
        }
    }
    let mesh = octree.generate(MeshParameters {
        boundary: Boundary {
            start: Vector::new([0, 0, 0]),
            end: Vector::new([64, 64, 64]),
        },
        lod: 1,
    });

    let root_path = root_path().expect("failed to get root path");

    let source_path = root_path.join("source");
    let asset_path = root_path.join("assets");

    let mut event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("FPS")
        .with_inner_size(winit::dpi::PhysicalSize { width: 1920, height: 1080 })
        .build(&event_loop)
        .unwrap();

    let mut resolution = window.inner_size().into();

    let (width, height) = resolution;

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
                present_mode: PresentMode::TripleBufferWaitForVBlank,
                ..default()
            })
            .expect("failed to create swapchain"),
    );

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
            color: &[gpu::prelude::Color {
                format: device.presentation_format(swapchain.get()).unwrap(),
                ..default()
            }],
            depth: Some(default()),
            raster: Raster {
                face_cull: FaceCull::FRONT,
                ..default()
            },
            ..default()
        })
        .expect("failed to create pipeline");

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
            size: REALLY_LARGE_SIZE,
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

    let index_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let color_buffer = device
        .create_buffer(BufferInfo {
            size: 1024,
            debug_name: "Color Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let camera_buffer = device
        .create_buffer(BufferInfo {
            size: 1024,
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
    let indices = mesh.indices();

    let colors = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ];

    let mut camera = Cell::new(Camera {
        fov: 90.0 * std::f32::consts::PI / 360.0,
        clip: (0.1, 1000.0),
        aspect_ratio: width as f32 / height as f32,
        position: Vector::new([32.0, -30.0, -32.0]),
        rotation: default(),
    });

    let camera_info = Cell::new(default());

    let updated = Cell::new(true);

    let mut game_input = Input::default();

    let vertex_buffer = || vertex_buffer;
    let index_buffer = || index_buffer;
    let color_buffer = || color_buffer;
    let camera_buffer = || camera_buffer;
    let staging_buffer = || staging_buffer;
    let depth_image = || depth_img.get();
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
    let mut fps_instant = startup_instant;

    let mut fps = 0;

    profiling::finish_frame!();

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let current_instant = time::Instant::now();

        let delta_time = current_instant.duration_since(last_instant).as_secs_f64();

        last_instant = current_instant;

        if cursor_captured {
            let mut position = camera.get().position;

            let pitch = camera.get().pitch();
            let roll = camera.get().roll();

            let mut movement = Vector::default();

            let mut diff = Matrix::identity();

            let mut direction = Vector::<f32, 3>::default();

            direction[0] += game_input.right as u8 as f32;
            direction[0] -= game_input.left as u8 as f32;
            direction[1] += game_input.down as u8 as f32;
            direction[1] -= game_input.up as u8 as f32;
            direction[2] += game_input.forward as u8 as f32;
            direction[2] -= game_input.backward as u8 as f32;

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

            movement *= speed * delta_time as f32;

            position += movement;

            camera.set(Camera {
                position,
                ..camera.get()
            });
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                camera_info.set(CameraInfo {
                    projection: camera.get().projection(),
                    view: camera.get().view(),
                    transform: camera.get().transform(),
                });

                if current_instant.duration_since(fps_instant).as_secs_f64() > 1.0 {
                    window.set_title(&format!("FPS {}", fps));
                    fps_instant = current_instant;
                    fps = 0;
                } else {
                    fps += 1;
                }

                (draw_executable.as_mut().unwrap())();
    
                profiling::finish_frame!();
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                window_id,
            } => {
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

                    let mut rotation = camera.get().rotation;

                    rotation[0] += x_rot as f32;
                    rotation[1] += y_rot as f32;

                    rotation[0] = rotation[0].clamp(
                        -std::f32::consts::PI / 2.0 + 0.1,
                        std::f32::consts::PI / 2.0 - 0.1,
                    );

                    camera.set(Camera {
                        rotation,
                        ..camera.get()
                    });
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { button, .. },
                window_id,
            } => {
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
                let Some(key_code) = input.virtual_keycode else {
                    return;
                };

                use winit::event::VirtualKeyCode::*;

                match key_code {
                    W => game_input.forward = input.state == winit::event::ElementState::Pressed,
                    A => game_input.left = input.state == winit::event::ElementState::Pressed,
                    S => game_input.backward = input.state == winit::event::ElementState::Pressed,
                    D => game_input.right = input.state == winit::event::ElementState::Pressed,
                    Space => game_input.up = input.state == winit::event::ElementState::Pressed,
                    LShift => game_input.down = input.state == winit::event::ElementState::Pressed,
                    Escape => {
                        cursor_captured = false;
                        window.set_cursor_icon(winit::window::CursorIcon::Default);
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::None)
                            .expect("could not grab mouse cursor");
                    }
                    _ => {}
                };
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id,
            } => {
                let winit::dpi::PhysicalSize { width, height } = size;

                resolution = (width, height);

                swapchain.set(
                    device
                        .create_swapchain(SwapchainInfo {
                            width,
                            height,
                            old_swapchain: Some(swapchain.get()),
                            present_mode: PresentMode::TripleBufferWaitForVBlank,
                            ..default()
                        })
                        .expect("failed to create swapchain"),
                );

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

                camera.set(Camera {
                    aspect_ratio: width as f32 / height as f32,
                    ..camera.get()
                });

                let mut draw_executor = device.create_executor(default());

                record_update(Update {
                    executor: &mut draw_executor,
                    vertices: &vertices,
                    indices: &indices,
                    colors: &colors,
                    camera_info: &camera_info,
                    updated: &updated,
                    staging_buffer: &staging_buffer,
                    vertex_buffer: &vertex_buffer,
                    index_buffer: &index_buffer,
                    color_buffer: &color_buffer,
                    camera_buffer: &camera_buffer,
                });

                record_draw(Draw {
                    executor: &mut draw_executor,
                    vertices: &vertices,
                    indices: &indices,
                    pipeline: &pipeline,
                    present_image: &present_image,
                    depth_image: &depth_image,
                    vertex_buffer: &vertex_buffer,
                    index_buffer: &index_buffer,
                    resolution,
                    color_buffer: &color_buffer,
                    camera_buffer: &camera_buffer,
                });

                draw_executor.submit(Submit {
                    wait_semaphore: &acquire_semaphore,
                    signal_semaphore: &present_semaphore,
                });

                draw_executor.present(Present {
                    swapchain: swapchain.get(),
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

pub const VERTEX_OFFSET: usize = 0;
pub const INDEX_OFFSET: usize = 65536;

struct Update<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub vertices: &'b [common::mesh::Vertex],
    pub indices: &'b [Index],
    pub colors: &'b [Color],
    pub camera_info: &'b Cell<CameraInfo>,
    pub updated: &'b Cell<bool>,
    pub vertex_buffer: &'b dyn ops::Fn() -> Buffer,
    pub index_buffer: &'b dyn ops::Fn() -> Buffer,
    pub staging_buffer: &'b dyn ops::Fn() -> Buffer,
    pub color_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
}

struct Draw<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub resolution: (u32, u32),
    pub present_image: &'b dyn ops::Fn() -> Image,
    pub depth_image: &'b dyn ops::Fn() -> Image,
    pub vertex_buffer: &'b dyn ops::Fn() -> Buffer,
    pub index_buffer: &'b dyn ops::Fn() -> Buffer,
    pub color_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
    pub pipeline: &'b Pipeline<'b>,
    pub vertices: &'b [common::mesh::Vertex],
    pub indices: &'b [Index],
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C)]
struct CameraInfo {
    projection: Matrix<f32, 4, 4>,
    transform: Matrix<f32, 4, 4>,
    view: Matrix<f32, 4, 4>,
}

#[derive(Clone, Copy, Default, Debug)]
struct Input {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Push {
    pub color_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
}

fn record_draw<'a, 'b: 'a>(draw: Draw<'a, 'b>) {
    use Resource::*;

    let Draw {
        executor,
        present_image,
        depth_image,
        vertex_buffer,
        index_buffer,
        color_buffer,
        camera_buffer,
        resolution,
        vertices,
        indices,
        pipeline,
    } = draw;

    executor.add(Task {
        resources: [
            Image(present_image, ImageAccess::ColorAttachment),
            Image(depth_image, ImageAccess::DepthStencilAttachment),
            Buffer(color_buffer, BufferAccess::ShaderReadOnly),
            Buffer(camera_buffer, BufferAccess::ShaderReadOnly),
            Buffer(vertex_buffer, BufferAccess::ShaderReadOnly),
            Buffer(index_buffer, BufferAccess::ShaderReadOnly),
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
                    clear: Clear::Depth(0.0),
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
                    color_buffer: (color_buffer)(),
                    camera_buffer: (camera_buffer)(),
                    vertex_buffer: (vertex_buffer)(),
                    index_buffer: (index_buffer)(),
                },
                pipeline: &pipeline,
            });

            commands.draw(gpu::prelude::Draw {
                vertex_count: vertices.len(),
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

fn record_update<'a, 'b: 'a>(update: Update<'a, 'b>) {
    use Resource::*;

    let Update {
        executor,
        staging_buffer,
        vertex_buffer,
        index_buffer,
        color_buffer,
        camera_buffer,
        vertices,
        indices,
        colors,
        updated,
        camera_info,
    } = update;

    executor.add(Task {
        resources: [
            Buffer(staging_buffer, BufferAccess::TransferRead),
            Buffer(camera_buffer, BufferAccess::TransferWrite),
            Buffer(color_buffer, BufferAccess::TransferWrite),
            Buffer(vertex_buffer, BufferAccess::TransferWrite),
            Buffer(index_buffer, BufferAccess::TransferWrite),
        ],
        task: move |commands| {
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

            if updated.get() {
                updated.set(false);
                dbg!("yo");
                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: 1024,
                    src: colors,
                })?;

                commands.write_buffer(BufferWrite {
                    buffer: 0,
                    offset: 2048,
                    src: vertices,
                })?;

                commands.copy_buffer_to_buffer(BufferCopy {
                    from: 0,
                    to: 2,
                    src: 1024,
                    dst: 0,
                    size: 1024,
                })?;

                commands.copy_buffer_to_buffer(BufferCopy {
                    from: 0,
                    to: 3,
                    src: 2048,
                    dst: 0,
                    size: 900_000_000,
                })?;
            }
            Ok(())
        },
    });
}
