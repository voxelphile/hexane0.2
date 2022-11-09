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
    pub fn is_transparent(&self) -> bool {
        match self.id {
            Id::Air => true,
            _ => false,
        }
    }
}
