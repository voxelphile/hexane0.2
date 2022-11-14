use math::prelude::*;

#[derive(Clone, Copy)]
pub enum Camera {
    Perspective {
        fov: f32,
        clip: (f32, f32),
        aspect_ratio: f32,
        position: Vector<f32, 3>,
        rotation: Vector<f32, 3>,
    },
    Orthographic {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        clip: (f32, f32),
        position: Vector<f32, 3>,
        rotation: Vector<f32, 3>,
    },
}

impl Camera {
    pub fn projection(&self) -> Matrix<f32, 4, 4> {
        match self {
            Camera::Perspective {
                fov,
                clip,
                aspect_ratio,
                ..
            } => {
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
            Camera::Orthographic {
                left,
                right,
                top,
                bottom,
                clip,
                ..
            } => {
                let mut projection = Matrix::<f32, 4, 4>::identity();

                let (near, far) = clip;

                projection[0][0] = 2.0 / (right - left);
                projection[1][1] = 2.0 / (bottom - top);
                projection[2][2] = 1.0 / (near - far);
                projection[3][0] = -(right + left) / (right - left);
                projection[3][1] = -(bottom + top) / (bottom - top);
                projection[3][2] = near / (near - far);

                projection
            }
        }
    }

    pub fn view(&self) -> Matrix<f32, 4, 4> {
        self.transform().inverse()
    }

    pub fn transform(&self) -> Matrix<f32, 4, 4> {
        let position = match self {
            Camera::Perspective { position, .. } => position,
            Camera::Orthographic { position, .. } => position,
        };

        let mut transform = Matrix::identity();

        transform = transform * self.roll();
        transform = transform * self.pitch();
        transform = transform * self.yaw();

        transform[3][0] = position[0];
        transform[3][1] = position[1];
        transform[3][2] = position[2];

        transform
    }

    pub fn yaw(&self) -> Matrix<f32, 4, 4> {
        let rotation = self.get_rotation();

        let mut yaw = Matrix::<f32, 4, 4>::identity();

        yaw[0][0] = rotation[2].cos();
        yaw[1][0] = -rotation[2].sin();
        yaw[0][1] = rotation[2].sin();
        yaw[1][1] = rotation[2].cos();

        yaw
    }

    pub fn pitch(&self) -> Matrix<f32, 4, 4> {
        let rotation = self.get_rotation();

        let mut pitch = Matrix::<f32, 4, 4>::identity();

        pitch[0][0] = rotation[1].cos();
        pitch[2][0] = rotation[1].sin();
        pitch[0][2] = -rotation[1].sin();
        pitch[2][2] = rotation[1].cos();

        pitch
    }

    pub fn roll(&self) -> Matrix<f32, 4, 4> {
        let rotation = self.get_rotation();

        let mut roll = Matrix::<f32, 4, 4>::identity();

        roll[1][1] = rotation[0].cos();
        roll[2][1] = -rotation[0].sin();
        roll[1][2] = rotation[0].sin();
        roll[2][2] = rotation[0].cos();

        roll
    }

    pub fn get_position(&self) -> Vector<f32, 3> {
        match self {
            Camera::Perspective { position, .. } => *position,
            Camera::Orthographic { position, .. } => *position,
        }
    }

    pub fn set_position(&mut self, position: Vector<f32, 3>) {
        *match self {
            Camera::Perspective { position, .. } => position,
            Camera::Orthographic { position, .. } => position,
        } = position;
    }

    pub fn get_rotation(&self) -> Vector<f32, 3> {
        match self {
            Camera::Perspective { rotation, .. } => *rotation,
            Camera::Orthographic { rotation, .. } => *rotation,
        }
    }

    pub fn set_rotation(&mut self, rotation: Vector<f32, 3>) {
        *match self {
            Camera::Perspective { rotation, .. } => rotation,
            Camera::Orthographic { rotation, .. } => rotation,
        } = rotation;
    }
}
