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
