#![feature(let_else)]

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use gpu::prelude::*;

fn main() {
    println!("Hello, world!");

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let (width, height) = window.inner_size().into();

    let context = Context::new(ContextInfo {
        enable_validation: true,
        application_name: "Hexane",
        engine_name: "Hexane",
        ..Default::default()
    })
    .expect("failed to create context");

    let device = context
        .create_device(DeviceInfo {
            window: &window,
            display: &window,
            ..Default::default()
        })
        .expect("failed to create device");

    let swapchain = device
        .create_swapchain(SwapchainInfo {
            width,
            height,
            ..Default::default()
        })
        .expect("failed to create swapchain");

    loop {}
}
