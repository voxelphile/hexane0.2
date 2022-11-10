use math::prelude::*;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: Vector<f32, 3>,
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

pub struct Boundary {
    pub start: Vector<usize, 3>,
    pub end: Vector<usize, 3>,
}

pub struct MeshParameters {
    pub boundary: Boundary,
    pub lod: usize,
}

pub trait MeshGenerator {
    fn generate(&self, parameters: MeshParameters) -> Mesh;
}
