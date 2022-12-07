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
const REALLY_LARGE_SIZE: usize = 500_000_000;

const CHUNK_SIZE: usize = 128;
const AXIS_MAX_CHUNKS: usize = 8;

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
    println!("Hello, client!");

    use common::net::*;

    let mut host = Host::connect("127.0.0.1:29757").unwrap();

    let mut i = 0;

    let mut instant = std::time::Instant::now();
    
    loop {
        if let Some(_) = host.drop() {
            println!("server dropped");
            break;
        }

        //It is crucial that we call receive because the first packet contains
        //a special authorization token
        let msgs = host.recv();

        if msgs.len() > 0 {
            dbg!(msgs);
        }

        if std::time::Instant::now().duration_since(instant).as_secs_f32() > 1.0 {
            dbg!("sent");
            host.send(0, Delivery::Reliable, Message::HelloWorld(i));
            i+= 1;
            instant = std::time::Instant::now();
        }
    }

    panic!("goodbye world");

    //tracy_client::Client::start();
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

    let mut resolution = Cell::new(window.inner_size().into());

    let (width, height) = resolution.get();
    /*
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
        /*
            let mesh: Mesh = octree.convert(Conversion {
                regions: [Region {
                    start: Vector::new([0, 0, 0]),
                    end: Vector::new([WORLD_SIZE, 128, WORLD_SIZE]),
                }],
                lod: 1,
            });

            println!("Finished mesh generation.");
        */
        let bitset: Bitset = octree.convert(Conversion {
            regions: [Region {
                start: Vector::new([0, 0, 0]),
                end: Vector::new([WORLD_SIZE, 128, WORLD_SIZE]),
            }],
            lod: 1,
        });

        dbg!(bitset.data().len() * mem::size_of::<u32>());

        println!("Finished bitset generation.");
    */
    let root_path = root_path().expect("failed to get root path");

    let source_path = root_path.join("source");
    let asset_path = root_path.join("assets");

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
        asset_path: &asset_path,
        ..default()
    });

    use ShaderType::*;

    let draw_pipeline = pipeline_compiler
        .create_graphics_pipeline(GraphicsPipelineInfo {
            shaders: [
                Shader(Vertex, "render_voxel", &[]),
                Shader(Fragment, "render_voxel", &[]),
            ],
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

    let input_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "input", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let world_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_world", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let bitset_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_bitset", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

    let vertex_pipeline = pipeline_compiler
        .create_compute_pipeline(ComputePipelineInfo {
            shader: Shader(Compute, "build_mesh", &[]),
            ..default()
        })
        .expect("failed to create pipeline");

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
                usage: ImageUsage::DEPTH,
                format: Format::D32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut depth_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::TwoDim(width as _, height as _),
                usage: ImageUsage::DEPTH,
                format: Format::D32Sfloat,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut noise_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::ThreeDim(16, 16, 16),
                usage: ImageUsage::TRANSFER_DST,
                format: Format::Rg32Uint,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut perlin_img = Cell::new(
        device
            .create_image(ImageInfo {
                extent: ImageExtent::ThreeDim(512, 512, 512),
                usage: ImageUsage::TRANSFER_DST,
                format: Format::R32Uint,
                ..default()
            })
            .expect("failed to create depth image"),
    );

    let mut chunk_images = vec![];

    let chunk_len = AXIS_MAX_CHUNKS.pow(3);

    for _ in 0..chunk_len {
        chunk_images.push(
            device
                .create_image(ImageInfo {
                    extent: ImageExtent::ThreeDim(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                    usage: ImageUsage::TRANSFER_DST,
                    format: Format::R16Uint,
                    ..default()
                })
                .expect("failed to create depth image"),
        );
    }

    let general_staging_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            memory: Memory::HOST_ACCESS,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let vertex_staging_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            memory: Memory::HOST_ACCESS,
            debug_name: "Staging Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let chunk_staging_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
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

    let vertex_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
            debug_name: "General Buffer",
            ..default()
        })
        .expect("failed to create buffer");

    let world_buffer = device
        .create_buffer(BufferInfo {
            size: REALLY_LARGE_SIZE,
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
            size: SMALL_SIZE,
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
        clip: (0.001, 500.0),
        aspect_ratio: width as f32 / height as f32,
        position: Vector::new([16.0, 48.0, 16.0]),
        rotation: default(),
    });

    let camera_info = Cell::new(default());

    camera_info.set(CameraInfo {
        projection: camera.get().projection(),
    });

    let info = Cell::new(Info::default());

    let update = Cell::new(true);
    let build = Cell::new(true);

    let physics_time_accum = Cell::new(0.0);

    let vertex_buffer = || vertex_buffer;
    let input_buffer = || input_buffer;
    let mersenne_buffer = || mersenne_buffer;
    let transform_buffer = || transform_buffer;
    let rigidbody_buffer = || rigidbody_buffer;
    let info_buffer = || info_buffer;
    let camera_buffer = || camera_buffer;
    let world_buffer = || world_buffer;
    let general_staging_buffer = || general_staging_buffer;
    let vertex_staging_buffer = || vertex_staging_buffer;
    let chunk_staging_buffer = || chunk_staging_buffer;
    let noise_staging_buffer = || noise_staging_buffer;
    let depth_image = || depth_img.get();
    let noise_image = || noise_img.get();
    let perlin_image = || perlin_img.get();
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
            delta_time: info.get().delta_time + delta_time as f32,
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

                if let Some(e) = &executable {
                    window.set_title(&format!("Hexane | FPS {}", e.fps()));
                }

                if !cursor_captured {
                    info.set(Info {
                        entity_input: default(),
                        ..info.get()
                    });
                }

                physics_time_accum.set(physics_time_accum.get() + info.get().delta_time);

                (executable.as_mut().unwrap())();

                info.set(Info {
                    entity_input: EntityInput {
                        look: default(),
                        ..info.get().entity_input
                    },
                    delta_time: default(),
                    ..info.get()
                });

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
                            .refresh_graphics_pipeline(&draw_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&input_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&vertex_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&noise_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&perlin_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&world_pipeline)
                            .unwrap();
                        pipeline_compiler
                            .refresh_compute_pipeline(&bitset_pipeline)
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

                resolution.set((width, height));

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
                            usage: ImageUsage::DEPTH,
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
                        })
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
                        if update.get() {
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
                        if update.get() {
                            commands.copy_buffer_to_buffer(BufferCopy {
                                from: 0,
                                to: 1,
                                src: 8192,
                                dst: 0,
                                size: mem::size_of::<CameraInfo>(),
                            })?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [Buffer(
                        &chunk_staging_buffer,
                        BufferAccess::HostTransferWrite,
                    )],
                    task: |commands| {
                        if update.get() {
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
                        if update.get() {
                            commands.copy_buffer_to_buffer(BufferCopy {
                                from: 0,
                                to: 1,
                                src: 0,
                                dst: 0,
                                size: REALLY_LARGE_SIZE,
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
                        if update.get() {
                            const W: u32 = 32;
                            const N: u32 = 642;

                            const F: u32 = 1812433253;

                            let index = N;
                            let mut mt = vec![];

                            mt.push(42069);

                            for i in 1..N as usize {
                                mt.push(F * (mt[i - 1] ^ (mt[i - 1] >> (W - 2))) + i as u32);
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
                        if update.get() {
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
                        if update.get() {
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
                                (size as f64 / WORK_GROUP_SIZE as f64).ceil() as usize;

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
                        if update.get() {
                            commands.set_pipeline(&perlin_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildPerlinPush {
                                    noise_image: (noise_image)(),
                                    perlin_image: (perlin_image)(),
                                },
                                pipeline: &perlin_pipeline,
                            })?;

                            let size = AXIS_MAX_CHUNKS * CHUNK_SIZE;

                            const WORK_GROUP_SIZE: usize = 8;

                            let dispatch_size =
                                (2048 as f64 / WORK_GROUP_SIZE as f64).ceil() as usize;

                            commands.dispatch(dispatch_size, dispatch_size, dispatch_size)?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if update.get() {
                            commands.set_pipeline(&world_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildWorldPush {
                                    perlin_image: (perlin_image)(),
                                    world_buffer: (world_buffer)(),
                                },
                                pipeline: &world_pipeline,
                            })?;

                            const WORK_GROUP_SIZE: usize = 8;

                            let size = (AXIS_MAX_CHUNKS * CHUNK_SIZE) / WORK_GROUP_SIZE;

                            commands.dispatch(size, size, size)?;
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&world_buffer, BufferAccess::ComputeShaderReadOnly),
                        Buffer(&vertex_buffer, BufferAccess::ComputeShaderReadWrite),
                        Image(&perlin_image, ImageAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        if update.get() {
                            commands.set_pipeline(&vertex_pipeline)?;

                            commands.push_constant(PushConstant {
                                data: BuildMeshPush {
                                    world_buffer: (world_buffer)(),
                                    vertex_buffer: (vertex_buffer)(),
                                    perlin_image: (perlin_image)(),
                                },
                                pipeline: &vertex_pipeline,
                            })?;

                            const WORK_GROUP_SIZE: usize = 8;

                            let dispatch_size = ((AXIS_MAX_CHUNKS * CHUNK_SIZE) as f64
                                / WORK_GROUP_SIZE as f64)
                                .ceil() as usize;

                            commands.dispatch(dispatch_size, dispatch_size, dispatch_size)?;
                            update.set(false);
                            build.set(false);
                        }
                        Ok(())
                    },
                });

                executor.add(Task {
                    resources: [
                        Buffer(&info_buffer, BufferAccess::ComputeShaderReadOnly),
                        Buffer(&input_buffer, BufferAccess::ComputeShaderReadOnly),
                        Buffer(&transform_buffer, BufferAccess::ComputeShaderReadWrite),
                        Buffer(&rigidbody_buffer, BufferAccess::ComputeShaderReadWrite),
                    ],
                    task: |commands| {
                        commands.set_pipeline(&input_pipeline)?;

                        commands.push_constant(PushConstant {
                            data: InputPush {
                                info_buffer: (info_buffer)(),
                                transform_buffer: (transform_buffer)(),
                                rigidbody_buffer: (rigidbody_buffer)(),
                                input_buffer: (input_buffer)(),
                            },
                            pipeline: &input_pipeline,
                        })?;

                        commands.dispatch(1, 1, 1)
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
                        Image(&present_image, ImageAccess::ColorAttachment),
                        Image(&depth_image, ImageAccess::DepthAttachment),
                        Buffer(&camera_buffer, BufferAccess::VertexShaderReadOnly),
                        Buffer(&vertex_buffer, BufferAccess::VertexShaderReadOnly),
                        Buffer(&transform_buffer, BufferAccess::VertexShaderReadOnly),
                        Buffer(&world_buffer, BufferAccess::FragmentShaderReadOnly),
                    ],
                    task: |commands| {
                        let (width, height) = resolution.get();

                        commands.set_resolution(resolution.get())?;

                        commands.set_pipeline(&draw_pipeline)?;

                        commands.start_rendering(Render {
                            color: [Attachment {
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

                        commands.push_constant(PushConstant {
                            data: DrawPush {
                                info_buffer: (info_buffer)(),
                                camera_buffer: (camera_buffer)(),
                                vertex_buffer: (vertex_buffer)(),
                                transform_buffer: (transform_buffer)(),
                                world_buffer: (world_buffer)(),
                            },
                            pipeline: &draw_pipeline,
                        })?;

                        const VERTICES_PER_CUBE: usize = 6;

                        commands.draw(gpu::prelude::Draw {
                            vertex_count: 2500000 * VERTICES_PER_CUBE,
                        })?;

                        commands.end_rendering()
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
pub struct DrawPush {
    pub info_buffer: Buffer,
    pub camera_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub world_buffer: Buffer,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InputPush {
    pub info_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub rigidbody_buffer: Buffer,
    pub input_buffer: Buffer,
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PhysicsPush {
    pub fixed_time: f32,
    pub info_buffer: Buffer,
    pub transform_buffer: Buffer,
    pub rigidbody_buffer: Buffer,
    pub world_buffer: Buffer,
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
    pub perlin_image: Image,
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
