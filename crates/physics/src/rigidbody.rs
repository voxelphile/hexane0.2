use math::prelude::*;
struct Rigidbody {
    linear_velocity: Vector<f32, 3>,
    linear_acceleration: Vector<f32, 3>,
    angular_velocity: Vector<f32, 3>,
    angular_acceleration: Vector<f32, 3>,
    mass: f32,
}
