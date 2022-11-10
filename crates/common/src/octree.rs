use crate::mesh::*;
use crate::voxel::*;

use math::prelude::*;

use std::default::default;
use std::marker;

//TODO change this with dynamic size
const SIZE: usize = 2;

#[derive(Debug)]
pub enum Error {
    NodeNotFound,
    InvalidMask,
}

pub trait Octree<T> {
    fn new() -> Self;

    fn place(&mut self, position: Vector<usize, 3>, object: T);
}

pub struct SparseOctree<T: Eq + Copy + Default> {
    size: usize,
    nodes: Vec<Node<T>>,
    data: marker::PhantomData<[T]>,
}

impl<T: Eq + Copy + Default> Octree<T> for SparseOctree<T> {
    fn new() -> Self {
        let size = SIZE;

        let mut nodes = vec![];

        nodes.push(Node {
            morton: 0,
            child: u32::MAX,
            valid: 0,
            data: default(),
        });

        Self {
            size,
            nodes,
            data: marker::PhantomData,
        }
    }

    fn place(&mut self, position: Vector<usize, 3>, data: T) {
        let [x, y, z] = <[usize; 3]>::from(*position);

        let hierarchy = self.get_position_hierarchy(x, y, z);

        let node = self.add_node(&hierarchy);

        node.unwrap().data = data;
    }
}

impl<T: Eq + Copy + Default> SparseOctree<T> {
    fn add_node(&mut self, hierarchy: &[u8]) -> Result<&'_ mut Node<T>, Error> {
        let mut index = 0;

        for (level, &mask) in hierarchy.iter().enumerate() {
            let child_offset = (self.nodes[index].valid & (mask as u32 - 1)).count_ones();

            let mask_valid = self.nodes[index].valid & mask as u32 == mask as u32;
            let child_exists = self.nodes[index].child != u32::MAX;

            if mask_valid && child_exists {
                let child_index = self.nodes[index].child as usize;

                index = child_index + child_offset as usize;
            } else {
                let node = self.nodes[index];

                self.nodes[index].valid |= mask as u32;

                let child_offset = (self.nodes[index].valid & (mask as u32 - 1)).count_ones();
                let precedent = self.nodes[index].valid.count_ones() - 1;

                let old_family = node.child;
                let new_family = self.nodes.len() as _;

                self.nodes[index].child = new_family;

                for sibling_offset in 0..precedent {
                    let new_index = new_family + sibling_offset;
                    let old_index = old_family + sibling_offset;
                    let sibling = self.nodes[old_index as usize];
                    self.nodes[old_index as usize] = default();
                    self.nodes.insert(new_index as usize, sibling);
                }

                let new_child = new_family + child_offset;

                let new_node = Node {
                    data: node.data,
                    ..default()
                };

                self.nodes.insert(new_child as usize, new_node);

                index = new_child as usize;
            }
        }

        let node = self
            .get_node_by_index_mut(index)
            .ok_or(Error::NodeNotFound)?;

        Ok(node)
    }

    pub fn get_morton_code(hierarchy: &[u8]) -> u64 {
        const MASK: u64 = 0x7;

        let mut morton = MASK;

        for &mask in hierarchy {
            morton <<= 3;

            let mut index = mask.trailing_zeros() as u64;

            morton |= index & MASK;
        }

        morton
    }

    pub fn optimize(&mut self) {
        let mut nodes = self.nodes.clone();

        let node_order = 0..nodes.len();

        for i in node_order.clone().rev() {
            let node = nodes[i];

            let child_invalid = node.child == u32::MAX;
            let partial_solid = node.valid != u8::MAX as u32;

            if child_invalid || partial_solid {
                continue;
            }

            let child_index = node.child as usize;

            let child_order = child_index..child_index + 8;

            let first_child = nodes[child_index];

            let all_children_same = nodes[child_order.clone()]
                .iter()
                .all(|node| node.data == first_child.data);

            if all_children_same {
                nodes[i] = Node {
                    morton: node.morton,
                    child: u32::MAX,
                    valid: 0,
                    ..first_child
                };

                for j in child_order {
                    nodes[j] = default();
                }
            }
        }

        nodes.retain(|node| node.morton != u64::MAX);

        nodes.sort_by(|a, b| a.morton.cmp(&b.morton));

        for i in node_order {
            let node = nodes[i];

            let child_invalid = node.child == u32::MAX;

            if child_invalid {
                continue;
            }

            let child = self.nodes[node.child as usize];

            nodes[i].child = nodes
                .binary_search_by(|probe| probe.morton.cmp(&child.morton))
                .expect("failed to find child") as _;
        }

        self.nodes = nodes;
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn get_position_hierarchy(&self, mut x: usize, mut y: usize, mut z: usize) -> Vec<u8> {
        let mut hierarchy = vec![];

        let mut cursor = 2usize.pow(self.size as _);

        for i in 0..self.size {
            cursor /= 2;

            let px = (x >= cursor) as u8;
            let py = (y >= cursor) as u8;
            let pz = (z >= cursor) as u8;

            let index = px * 4 + py * 2 + pz;

            let mask = 1 << index;

            x -= px as usize * cursor;
            y -= py as usize * cursor;
            z -= pz as usize * cursor;

            hierarchy.push(mask);
        }

        hierarchy
    }

    pub fn nodes(&self) -> &'_ [Node<T>] {
        &self.nodes
    }

    pub fn get_node_or_parent(&self, hierarchy: &[u8]) -> Result<(&'_ Node<T>, usize), Error> {
        let order = (0..hierarchy.len()).rev();

        for i in order {
            if let Ok(data) = self.get_node(&hierarchy[..i]) {
                return Ok(data);
            }
        }

        Err(Error::NodeNotFound)?
    }

    pub fn get_node_or_parent_mut(
        &mut self,
        hierarchy: &[u8],
    ) -> Result<(&'_ mut Node<T>, usize), Error> {
        let (_, index) = self.get_node_or_parent(&hierarchy)?;

        let node = self
            .get_node_by_index_mut(index)
            .ok_or(Error::NodeNotFound)?;

        Ok((node, index))
    }

    pub fn get_node(&self, hierarchy: &[u8]) -> Result<(&'_ Node<T>, usize), Error> {
        let mut index = 0;

        for (level, &mask) in hierarchy.iter().enumerate() {
            if mask.count_ones() != 1 {
                Err(Error::InvalidMask)?
            }

            let child_offset = (self.nodes[index].valid & (mask as u32 - 1)).count_ones();

            let mask_valid = self.nodes[index].valid & mask as u32 == mask as u32;
            let child_exists = self.nodes[index].child != u32::MAX;

            if mask_valid && child_exists {
                let child_index = self.nodes[index].child as usize;

                index = child_index + child_offset as usize;
            } else {
                Err(Error::NodeNotFound)?
            }
        }

        let node = self.get_node_by_index(index).ok_or(Error::NodeNotFound)?;

        Ok((node, index))
    }

    pub fn get_node_mut(&mut self, hierarchy: &[u8]) -> Result<(&'_ Node<T>, usize), Error> {
        let (_, index) = self.get_node(&hierarchy)?;

        let node = self
            .get_node_by_index_mut(index)
            .ok_or(Error::NodeNotFound)?;

        Ok((node, index))
    }

    pub fn get_node_by_index(&self, index: usize) -> Option<&'_ Node<T>> {
        self.nodes.get(index)
    }

    pub fn get_node_by_index_mut(&mut self, index: usize) -> Option<&'_ mut Node<T>> {
        self.nodes.get_mut(index)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Node<T: Eq + Copy + Default> {
    child: u32,
    valid: u32,
    morton: u64,
    data: T,
}

impl<T: Eq + Copy + Default> Node<T> {
    pub fn child(&self) -> u32 {
        self.child
    }

    pub fn valid(&self) -> u32 {
        self.valid
    }

    pub fn morton(&self) -> u64 {
        self.morton
    }

    pub fn data(&self) -> T {
        self.data
    }
}

impl<T: Eq + Copy + Default> Default for Node<T> {
    fn default() -> Self {
        Node {
            child: u32::MAX,
            valid: 0,
            morton: u64::MAX,
            data: default(),
        }
    }
}

impl MeshGenerator for SparseOctree<Voxel> {
    fn generate(&self, parameters: MeshParameters) -> Mesh {
        let MeshParameters { boundary, lod } = parameters;

        let Boundary { start, end } = boundary;

        let [sx, sy, sz] = <[usize; 3]>::from(*start);
        let [ex, ey, ez] = <[usize; 3]>::from(*end);

        let mut vertices = vec![];

        let mut indices = vec![];

        for x in sx..ex {
            for y in sy..ey {
                for z in sz..ez {
                    let hierarchy = self.get_position_hierarchy(x, y, z);

                    let Ok(node) = self.get_node_or_parent(&hierarchy) else {
                        continue;
                    };

                    let neighbor_order = (-1isize..=1);

                    dbg!("yo");
                    for i in neighbor_order.clone() {
                        for j in neighbor_order.clone() {
                            for k in neighbor_order.clone() {
                                if isize::abs(i) + isize::abs(j) + isize::abs(k) != 1 {
                                    continue;
                                }
                                dbg!((i, j, k));
                                let (a, b, c) = (
                                    (x as isize + i) as usize,
                                    (y as isize + j) as usize,
                                    (z as isize + k) as usize,
                                );

                                let hierarchy = self.get_position_hierarchy(a, b, c);

                                let Err(_) = self.get_node(&hierarchy) else {
                                    continue;
                                };

                                dbg!("y0");

                                let p1 = Vector::new([0.0, 0.0, 0.0, 1.0]);
                                let p2 = Vector::new([0.0, 1.0, 0.0, 1.0]);
                                let p3 = Vector::new([1.0, 1.0, 0.0, 1.0]);
                                let p4 = Vector::new([1.0, 0.0, 0.0, 1.0]);

                                let uyz = Vector::new([j as f32, k as f32]);
                                let vyz = Vector::new([0.0, 1.0]);

                                let uxz = Vector::new([i as f32, k as f32]);
                                let vxz = Vector::new([0.0, 1.0]);

                                let uxy = Vector::new([i as f32, j as f32]);
                                let vxy = Vector::new([0.0, 0.0]);

                                let rotation = Vector::new([
                                    f32::acos(uyz.dot(vyz)),
                                    f32::acos(uxz.dot(vxz)),
                                    f32::acos(uxy.dot(vxy)),
                                ]);

                                let mut yaw = Matrix::<f32, 4, 4>::identity();

                                yaw[0][0] = rotation[2].cos();
                                yaw[1][0] = -rotation[2].sin();
                                yaw[0][1] = rotation[2].sin();
                                yaw[1][1] = rotation[2].cos();

                                let mut pitch = Matrix::<f32, 4, 4>::identity();

                                pitch[0][0] = rotation[1].cos();
                                pitch[2][0] = rotation[1].sin();
                                pitch[0][2] = -rotation[1].sin();
                                pitch[2][2] = rotation[1].cos();

                                let mut roll = Matrix::<f32, 4, 4>::identity();

                                roll[1][1] = rotation[0].cos();
                                roll[2][1] = -rotation[0].sin();
                                roll[1][2] = rotation[0].sin();
                                roll[2][2] = rotation[0].cos();

                                let mut transform = yaw * pitch * roll;

                                transform[3][0] = i as f32;
                                transform[3][1] = j as f32;
                                transform[3][2] = k as f32;

                                let p1 = transform * p1;
                                let p2 = transform * p2;
                                let p3 = transform * p3;
                                let p4 = transform * p4;

                                let p1 = Vector::new([p1[0], p1[1], p1[2]]);
                                let p2 = Vector::new([p2[0], p2[1], p2[2]]);
                                let p3 = Vector::new([p3[0], p3[1], p3[2]]);
                                let p4 = Vector::new([p4[0], p4[1], p4[2]]);

                                vertices.push(Vertex { position: p1 });
                                vertices.push(Vertex { position: p2 });
                                vertices.push(Vertex { position: p3 });

                                vertices.push(Vertex { position: p1 });
                                vertices.push(Vertex { position: p3 });
                                vertices.push(Vertex { position: p4 });

                                let q = vertices.len() as u32;

                                indices.push(q);
                                indices.push(q + 1);
                                indices.push(q + 2);
                                indices.push(q + 3);
                                indices.push(q + 4);
                                indices.push(q + 5);
                            }
                        }
                    }
                }
            }
        }

        Mesh::new(&vertices, &indices)
    }
}
