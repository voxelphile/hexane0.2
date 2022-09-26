fn main() {
    println!("Hello, world!");

    let context = gpu::Context::new(gpu::ContextInfo {
        enable_validation: true,
        application_name: "Hexane",
        engine_name: "Hexane",
        ..Default::default()
    })
    .expect("failed to create context");
}
