use math::prelude::*;

pub trait Transform<T> {
    fn transform<const N: usize>(&self, transformation: Transformation<N>) -> T;
}

pub struct Transformation<const N: usize> {
    pub regions: [Region; N],
    pub lod: usize,
}

pub struct Region {
    pub start: Vector<usize, 3>,
    pub end: Vector<usize, 3>,
}
