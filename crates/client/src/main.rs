#![feature(let_else)]

use gpu::prelude::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use raw_window_handle::{HasRawWindowHandle, HasRawDisplayHandle};

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
            display: window.raw_display_handle(),
            window: window.raw_window_handle(),
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
