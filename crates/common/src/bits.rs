const U32_bits: usize = 32;

pub struct Bitset {
    size: usize,
    data: Vec<u32>,
}

impl Bitset {
    pub fn new() -> Self {
        Self {
            size: 0,
            data: vec![],
        }
    }
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn data(&self) -> &[u32] {
        &self.data
    }

    pub fn insert(&mut self, index: usize, value: bool) -> Result<bool, ()> {
        while self.size <= index {
            self.data.push(0);
            self.size = self.data.len() * U32_bits;
        }

        let previous = self.get(index)?;

        if value {
            self.data[index / U32_bits] |= 1 << index % U32_bits;
        } else {
            self.data[index / U32_bits] &= !(1 << index % U32_bits);
        }

        Ok(previous)
    }

    pub fn get(&self, index: usize) -> Result<bool, ()> {
        if self.size <= index {
            return Err(());
        }

        Ok(self.data[index / U32_bits] & (1 << index % U32_bits) != 0)
    }
}
