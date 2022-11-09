use math::prelude::*;

#[derive(Clone, Copy)]
pub struct Camera {
    pub fov: f32,
    pub clip: (f32, f32),
    pub aspect_ratio: f32,
    pub position: Vector<f32, 3>,
    pub rotation: Vector<f32, 3>,
}

impl Camera {
    pub fn projection(&self) -> Matrix<f32, 4, 4> {
        let Camera {
            fov,
            clip,
            aspect_ratio,
            ..
        } = self;

        let mut projection = Matrix::<f32, 4, 4>::identity();

        let (near, far) = clip;

        let focal_length = 1.0 / (fov / 2.0).tan();

        projection[0][0] = focal_length / aspect_ratio;
        projection[1][1] = -focal_length;
        projection[2][2] = far / (near - far);
        projection[2][3] = -1.0;
        projection[3][2] = (near * far) / (near - far);
        projection
    }

    pub fn view(&self) -> Matrix<f32, 4, 4> {
        self.transform().inverse()
    }

    pub fn transform(&self) -> Matrix<f32, 4, 4> {
        let Camera {
            position, rotation, ..
        } = self;

        let mut transform = Matrix::identity();

        transform[3][0] = position[0];
        transform[3][1] = position[1];
        transform[3][2] = position[2];

        let mut yaw = Matrix::<f32, 4, 4>::identity();
        let mut pitch = Matrix::<f32, 4, 4>::identity();
        let mut roll = Matrix::<f32, 4, 4>::identity();

        yaw[0][0] = rotation[2].cos();
        yaw[1][0] = -rotation[2].sin();
        yaw[0][1] = rotation[1].sin();
        yaw[1][1] = rotation[1].cos();

        pitch[0][0] = rotation[1].cos();
        pitch[2][0] = rotation[1].sin();
        pitch[0][2] = -rotation[1].sin();
        pitch[2][2] = rotation[1].cos();

        roll[1][1] = rotation[0].cos();
        roll[2][1] = -rotation[0].sin();
        roll[1][2] = rotation[0].sin();
        roll[2][2] = rotation[0].cos();

        transform = transform * yaw;
        transform = transform * pitch;
        transform = transform * roll;

        transform
    }
}
