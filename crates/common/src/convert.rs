use math::prelude::*;

pub trait Convert<T> {
    fn convert<const N: usize>(&self, conversion: Conversion<N>) -> T;
}

pub struct Conversion<const N: usize> {
    pub regions: [Region; N],
    pub lod: usize,
}

pub struct Region {
    pub start: Vector<usize, 3>,
    pub end: Vector<usize, 3>,
}
