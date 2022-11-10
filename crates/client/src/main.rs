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

const REALLY_LARGE_SIZE: usize = 1_000_000;

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

    let root_path = root_path().expect("failed to get root path");

    let source_path = root_path.join("source");
    let asset_path = root_path.join("assets");

    let mut event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Hello Triangle!")
        .build(&event_loop)
        .unwrap();

    let mut resolution = window.inner_size().into();

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

    let mut swapchain = Cell::new(
        device
            .create_swapchain(SwapchainInfo {
                width,
                height,
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

    let vertices = [(0.0, -0.5, 0.0), (0.5, 0.5, 0.0), (-0.5, 0.5, 0.0)];
    let colors = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ];
    let indices = [0u32, 1, 2];

    let mut camera = Cell::new(Camera {
        fov: 90.0 * std::f32::consts::PI / 360.0,
        clip: (0.1, 1000.0),
        aspect_ratio: width as f32 / height as f32,
        position: default(),
        rotation: default(),
    });

    let camera_info = Cell::new(default());

    let mut game_input = Input::default();

    let general_buffer = || general_buffer;
    let color_buffer = || color_buffer;
    let camera_buffer = || camera_buffer;
    let staging_buffer = || staging_buffer;

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

    let mut executable: Option<Executable<'_>> = None;

    let startup_instant = time::Instant::now();
    let mut last_instant = startup_instant;

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
            direction[1] += game_input.up as u8 as f32;
            direction[1] -= game_input.down as u8 as f32;
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
        
                (executable.as_mut().unwrap())();
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

                    let x_rot = -(y_diff * delta_time) / sens;
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
                            ..default()
                        })
                        .expect("failed to create swapchain"),
                );

                camera.set(Camera {
                    aspect_ratio: width as f32 / height as f32,
                    ..camera.get()
                });

                let mut executor = device.create_executor(default());

                record(Record {
                    executor: &mut executor,
                    swapchain: swapchain.get(),
                    vertices: &vertices,
                    indices: &indices,
                    acquire_semaphore: &acquire_semaphore,
                    present_semaphore: &present_semaphore,
                    pipeline: &pipeline,
                    present_image: &present_image,
                    general_buffer: &general_buffer,
                    staging_buffer: &staging_buffer,
                    color_buffer: &color_buffer,
                    camera_buffer: &camera_buffer,
                    colors: &colors,
                    camera_info: &camera_info,
                    resolution,
                });

                executable = Some(executor.complete().expect("failed to complete executor"));
            }
            _ => (),
        }
    });
}

pub const VERTEX_OFFSET: usize = 0;
pub const INDEX_OFFSET: usize = 1024;

struct Record<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub swapchain: Swapchain,
    pub vertices: &'b [Vertex],
    pub indices: &'b [Index],
    pub colors: &'b [Color],
    pub camera_info: &'b Cell<CameraInfo>,
    pub present_image: &'b dyn ops::Fn() -> Image,
    pub general_buffer: &'b dyn ops::Fn() -> Buffer,
    pub staging_buffer: &'b dyn ops::Fn() -> Buffer,
    pub color_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
    pub acquire_semaphore: &'b BinarySemaphore<'b>,
    pub present_semaphore: &'b BinarySemaphore<'b>,
    pub pipeline: &'b Pipeline<'b>,
    pub resolution: (u32, u32),
}

struct Update<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub vertices: &'b [Vertex],
    pub indices: &'b [Index],
    pub colors: &'b [Color],
    pub camera_info: &'b Cell<CameraInfo>,
    pub general_buffer: &'b dyn ops::Fn() -> Buffer,
    pub staging_buffer: &'b dyn ops::Fn() -> Buffer,
    pub color_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
}

struct Draw<'a, 'b: 'a> {
    pub executor: &'a mut Executor<'b>,
    pub resolution: (u32, u32),
    pub present_image: &'b dyn ops::Fn() -> Image,
    pub general_buffer: &'b dyn ops::Fn() -> Buffer,
    pub color_buffer: &'b dyn ops::Fn() -> Buffer,
    pub camera_buffer: &'b dyn ops::Fn() -> Buffer,
    pub pipeline: &'b Pipeline<'b>,
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
}

fn record<'a, 'b: 'a>(record: Record<'a, 'b>) {
    let Record {
        executor,
        swapchain,
        vertices,
        indices,
        colors,
        camera_info,
        general_buffer,
        staging_buffer,
        color_buffer,
        camera_buffer,
        present_image,
        pipeline,
        acquire_semaphore,
        present_semaphore,
        resolution,
    } = record;

    record_update(Update {
        executor,
        vertices: &vertices,
        indices: &indices,
        colors: &colors,
        camera_info: &camera_info,
        general_buffer,
        staging_buffer,
        color_buffer,
        camera_buffer,
    });

    record_draw(Draw {
        executor,
        indices: &indices,
        pipeline: &pipeline,
        present_image,
        general_buffer,
        resolution,
        color_buffer,
        camera_buffer,
    });

    executor.submit(Submit {
        wait_semaphore: &acquire_semaphore,
        signal_semaphore: &present_semaphore,
    });

    executor.present(Present {
        swapchain,
        wait_semaphore: &present_semaphore,
    });
}

fn record_draw<'a, 'b: 'a>(draw: Draw<'a, 'b>) {
    use Resource::*;

    let Draw {
        executor,
        present_image,
        general_buffer,
        color_buffer,
        camera_buffer,
        resolution,
        indices,
        pipeline,
    } = draw;

    executor.add(Task {
        resources: [
            Image(present_image, ImageAccess::ColorAttachment),
            Buffer(color_buffer, BufferAccess::ShaderReadOnly),
            Buffer(camera_buffer, BufferAccess::ShaderReadOnly),
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
                    src_access: Access::None,
                    dst_access: Access::ColorAttachmentWrite,
                }],
            })?;

            commands.set_resolution(resolution)?;

            commands.start_rendering(Render {
                color: &[Attachment {
                    image: 0,
                    load_op: LoadOp::Clear,
                    clear: Clear::Color(0.1, 0.4, 0.8, 1.0),
                }],
                depth: None,
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
                },
                pipeline: &pipeline,
            });

            commands.draw(gpu::prelude::Draw { vertex_count: 3 })?;

            commands.end_rendering()?;

            commands.pipeline_barrier(PipelineBarrier {
                src_stage: PipelineStage::ColorAttachmentOutput,
                dst_stage: PipelineStage::BottomOfPipe,
                barriers: &[Barrier::Image {
                    image: 0,
                    old_layout: ImageLayout::ColorAttachmentOptimal,
                    new_layout: ImageLayout::Present,
                    src_access: Access::ColorAttachmentWrite,
                    dst_access: Access::None,
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
        general_buffer,
        color_buffer,
        camera_buffer,
        vertices,
        indices,
        colors,
        camera_info,
    } = update;

    executor.add(Task {
        resources: [Buffer(staging_buffer, BufferAccess::HostTransferWrite)],
        task: move |commands| {
            commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: 0,
                src: &[camera_info.get()],
            })
        },
    });

    executor.add(Task {
        resources: [
            Buffer(staging_buffer, BufferAccess::TransferRead),
            Buffer(camera_buffer, BufferAccess::TransferWrite),
        ],
        task: move |commands| {
            commands.copy_buffer_to_buffer(BufferCopy {
                from: 0,
                to: 1,
                src: 0,
                dst: 0,
                size: mem::size_of::<CameraInfo>(),
            })
        },
    });

    executor.add(Task {
        resources: [Buffer(staging_buffer, BufferAccess::HostTransferWrite)],
        task: move |commands| {
            commands.write_buffer(BufferWrite {
                buffer: 0,
                offset: 65536,
                src: colors,
            })
        },
    });

    executor.add(Task {
        resources: [
            Buffer(staging_buffer, BufferAccess::TransferRead),
            Buffer(color_buffer, BufferAccess::TransferWrite),
        ],
        task: move |commands| {
            commands.copy_buffer_to_buffer(BufferCopy {
                from: 0,
                to: 1,
                src: 65536,
                dst: 0,
                size: 1024,
            })
        },
    });
}
