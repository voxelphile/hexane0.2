use math::prelude::*;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: Vector<f32, 4>,
    pub normal: Vector<f32, 4>,
    pub color: Vector<f32, 4>,
    pub ambient: Vector<f32, 4>,
}

pub type Index = u32;

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<Index>,
}

impl Mesh {
    pub fn new(vertices: &[Vertex], indices: &[Index]) -> Self {
        Self {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
        }
    }
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    pub fn indices(&self) -> &[Index] {
        &self.indices
    }
}
