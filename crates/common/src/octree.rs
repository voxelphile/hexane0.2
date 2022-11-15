use crate::bits::*;
use crate::mesh::*;
use crate::transform::*;
use crate::voxel::*;

use math::prelude::*;

use std::cmp;
use std::default::default;
use std::iter;
use std::marker;

//TODO change this with dynamic size
const SIZE: usize = 10;

#[derive(Debug)]
pub enum Error {
    NodeNotFound,
    NoData,
    InvalidMask,
}

pub trait Octree<T> {
    fn new() -> Self;

    fn place(&mut self, position: Vector<usize, 3>, object: T);
    fn query(&self, position: Vector<usize, 3>) -> Option<&T>;
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

        node.unwrap().data = Some(data);
    }

    fn query(&self, position: Vector<usize, 3>) -> Option<&T> {
        let [x, y, z] = <[usize; 3]>::from(*position);

        let hierarchy = self.get_position_hierarchy(x, y, z);

        let Ok((node, _)) = self.get_node(&hierarchy) else {
            None?
        };
    
        let Some(data) = &node.data else {
            None?
        };

        Some(data)
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
                    morton: Self::get_morton_code(&hierarchy[..=level]),
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

        for i in node_order.rev() {
            let child_invalid = nodes[i].child == u32::MAX;
            let partial_solid = nodes[i].valid != u8::MAX as u32;

            if child_invalid || partial_solid {
                continue;
            }

            let child_index = nodes[i].child as usize;

            let child_order = child_index..child_index + 8;

            let all_children_same = nodes[child_order.clone()]
                .iter()
                .all(|node| node.data.is_some() && node.data == nodes[child_index].data);

            if all_children_same {
                nodes[i] = Node {
                    morton: nodes[i].morton,
                    data: nodes[child_index].data,
                    ..default()
                };

                for j in child_order {
                    nodes[j] = default();
                }
            }
        }

        nodes.retain(|node| node.morton != u64::MAX);

        nodes.sort_by(|a, b| a.morton.cmp(&b.morton));
        
        let node_order = 0..nodes.len();

        for i in node_order {
            let child_invalid = nodes[i].child == u32::MAX;

            if child_invalid {
                continue;
            }

            let child = self.nodes.get(nodes[i].child as usize).expect(&format!("{}", nodes[i].child));
            
                nodes[i].child = nodes.binary_search_by(|probe| probe.morton.cmp(&child.morton)).expect("failed to find child") as _;
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

            let octant = px * 4 + py * 2 + pz;

            let mask = 1 << octant;

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
    
    pub fn has_data(&self, hierarchy: &[u8]) -> bool {
        let Ok((node, _)) = self.get_node(&hierarchy) else {
            return false;
        };

        return node.data.is_some();
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
                break;
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
    data: Option<T>,
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

    pub fn data(&self) -> Option<T> {
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

impl Transform<Mesh> for SparseOctree<Voxel> {
    fn transform<const N: usize>(&self, transformation: Transformation<N>) -> Mesh {
        let Transformation { regions, lod } = transformation;

        let mut vertices = vec![];

        let mut indices = vec![];

        for region in regions {
            let Region { start, end } = region;

            let [sx, sy, sz] = *start;
            let [ex, ey, ez] = *end;

            for x in sx..ex {
                for y in sy..ey {
                    for z in sz..ez {
                        let hierarchy = self.get_position_hierarchy(x, y, z);

                        let Ok((node, index)) = self.get_node(&hierarchy) else {
                        continue;
                    };

                        if !self.has_data(&hierarchy) {
                            continue;
                        }

                        let position = Vector::new([x as f32, y as f32, z as f32, 1.0]);

                        let color = node.data.unwrap().albedo(position);

                        let vertex_ao = |side: Vector<f32, 2>, corner: f32| {
                            (side[0] + side[1] + f32::max(corner, side[0] * side[1])) / 3.0
                        };

                        let voxel_ao = |position: Vector<f32, 4>, normal: Vector<f32, 4>| {
                            let d1 = Vector::new([
                                isize::abs(normal[1] as isize) as usize,
                                isize::abs(normal[2] as isize) as usize,
                                isize::abs(normal[0] as isize) as usize,
                            ]);

                            let d2 = Vector::new([
                                isize::abs(normal[2] as isize) as usize,
                                isize::abs(normal[0] as isize) as usize,
                                isize::abs(normal[1] as isize) as usize,
                            ]);

                            let position = Vector::new([
                                position[0] as isize + normal[0] as isize,
                                position[1] as isize + normal[1] as isize,
                                position[2] as isize + normal[2] as isize,
                            ]);

                            let position = Vector::new([
                                position[0] as usize,
                                position[1] as usize,
                                position[2] as usize,
                            ]);

                            let side = Vector::new([
                                self.query(position + d1).is_some() as usize as f32,
                                self.query(position + d2).is_some() as usize as f32,
                                self.query(position - d1).is_some() as usize as f32,
                                self.query(position - d2).is_some() as usize as f32,
                            ]);

                            let corner = Vector::new([
                                self.query(position + d1 + d2).is_some() as usize as f32,
                                self.query(position - d1 + d2).is_some() as usize as f32,
                                self.query(position - d1 - d2).is_some() as usize as f32,
                                self.query(position + d1 - d2).is_some() as usize as f32,
                            ]);

                            Vector::new([
                                1.0 - vertex_ao(Vector::new([side[0], side[1]]), corner[0]),
                                1.0 - vertex_ao(Vector::new([side[1], side[2]]), corner[1]),
                                1.0 - vertex_ao(Vector::new([side[2], side[3]]), corner[2]),
                                1.0 - vertex_ao(Vector::new([side[3], side[0]]), corner[3]),
                            ])
                        };

                        let size = 2usize.pow(self.size as _);

                        let mut normals = vec![];

                        if z < size {
                            if !self.has_data(&self.get_position_hierarchy(x, y, z + 1))
                            {
                                normals.push(Vector::new([0.0, 0.0, 1.0, 0.0]));
                            }
                        } else {
                            normals.push(Vector::new([0.0, 0.0, 1.0, 0.0]));
                        }

                        if z > 0 {
                            if !self.has_data(&self.get_position_hierarchy(x, y, z - 1))
                            {
                                normals.push(Vector::new([0.0, 0.0, -1.0, 0.0]));
                            }
                        } else {
                            normals.push(Vector::new([0.0, 0.0, -1.0, 0.0]));
                        }

                        if x < size {
                            if !self.has_data(&self.get_position_hierarchy(x + 1, y, z))
                            {
                                normals.push(Vector::new([1.0, 0.0, 0.0, 0.0]));
                            }
                        } else {
                            normals.push(Vector::new([1.0, 0.0, 0.0, 0.0]));
                        }

                        if x > 0 {
                            if !self.has_data(&self.get_position_hierarchy(x - 1, y, z))
                            {
                                normals.push(Vector::new([-1.0, 0.0, 0.0, 0.0]));
                            }
                        } else {
                            normals.push(Vector::new([-1.0, 0.0, 0.0, 0.0]));
                        }

                        if y < size {
                            if !self.has_data(&self.get_position_hierarchy(x, y + 1, z))
                            {
                                normals.push(Vector::new([0.0, 1.0, 0.0, 0.0]));
                            }
                        } else {
                            normals.push(Vector::new([0.0, 1.0, 0.0, 0.0]));
                        }

                        if y > 0 {
                            if !self.has_data(&self.get_position_hierarchy(x, y - 1, z))
                            {
                                normals.push(Vector::new([0.0, -1.0, 0.0, 0.0]));
                            }
                        } else {
                            normals.push(Vector::new([0.0, -1.0, 0.0, 0.0]));
                        }

                        for normal in normals {
                            let ambient = voxel_ao(position, normal);

                            vertices.push(Vertex {
                                position,
                                color,
                                normal,
                                ambient,
                            });
                        }
                    }
                }
            }
        }

        Mesh::new(&vertices, &indices)
    }
}

impl Transform<Bitset> for SparseOctree<Voxel> {
    //TODO implement this for N regions
    fn transform<const N: usize>(&self, transformation: Transformation<N>) -> Bitset {
        let Transformation { regions, lod } = transformation;

        let Region { start, end } = regions[0];

        let [sx, sy, sz] = *start;
        let [ex, ey, ez] = *end;

        let mut bitset = Bitset::new();

        //root node is a given
        bitset.insert(0, true);

        for x in sx..ex {
            for y in sy..ey {
                for z in sz..ez {
                    let hierarchy = self.get_position_hierarchy(x, y, z);

                    for level in (0..hierarchy.len()).rev() {
                        let Ok((node, _)) = self.get_node(&hierarchy[..=level]) else {
                            break;
                        };

                        if !self.has_data(&hierarchy[..=level]) {
                            break;
                        }

                        let mut index = 0;

                        for i in 0..=level {
                            index += 8usize.pow(i as u32) as usize;
                        }

                        for (i, &mask) in hierarchy[..=level].iter().rev().enumerate() {
                            let previous = 8usize.pow(i as u32) as usize;
                            index += mask.trailing_zeros() as usize * previous;
                        }

                        bitset.insert(index, true).unwrap();
                    }
                }
            }
        }

        bitset
    }
}
