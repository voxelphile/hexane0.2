use math::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum Id {
    #[default]
    Error = 0,
    Air = 1,
    Grass = 2,
    Water = 3,
    Dirt = 4,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Voxel {
    pub id: Id,
}

impl Voxel {
    pub fn albedo(&self, position: Vector<f32, 4>) -> Vector<f32, 4> {
        use Id::*;

        use noise::{NoiseFn, Perlin, Seedable};

        let perlin = Perlin::new(420);

        let position =
            Vector::<f64, 3>::new([position[0] as f64, position[1] as f64, position[2] as f64]);

        let mix = (perlin.get(*(position / 64.0)) as f32 + 1.0) / 2.0;

        match self.id {
            Dirt => blend(
                Vector::new([107.0 / 256.0, 84.0 / 256.0, 40.0 / 256.0, 1.0]),
                Vector::new([64.0 / 256.0, 41.0 / 256.0, 5.0 / 256.0, 1.0]),
                mix,
            ),
            Grass => blend(
                Vector::new([170.0 / 256.0, 255.0 / 256.0, 21.0 / 256.0, 1.0]),
                Vector::new([34.0 / 256.0, 139.0 / 256.0, 34.0 / 256.0, 1.0]),
                mix,
            ),
            _ => Vector::new([1.0, 1.0, 1.0, 1.0]),
        }
    }

    pub fn is_transparent(&self) -> bool {
        match self.id {
            Id::Air => true,
            _ => false,
        }
    }
}

fn blend(from: Vector<f32, 4>, to: Vector<f32, 4>, mix: f32) -> Vector<f32, 4> {
    let lerp = |a, b, t| a + t * (b - a);

    Vector::new([
        lerp(from[0], to[0], mix),
        lerp(from[1], to[1], mix),
        lerp(from[2], to[2], mix),
        lerp(from[3], to[3], mix),
    ])
}
