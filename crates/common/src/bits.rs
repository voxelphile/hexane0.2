pub struct Bitset {
    size: u32,
    data: Vec<u32>,
}

impl Bitset {
    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn data(&self) -> &[u32] {
        &self.data
    }

    pub fn insert(&mut self, index: usize, value: bool) -> bool {
        const U32_bits: usize = 32;

        while self.size / U32_bits > index / U32_bits {
            self.data.push(0);
        }

        let previous = self.get(index);

        if value {
            self.data[index / U32_bits] |= 1 << index % U32_bits; 
        } else {
            self.data[index / U32_bits] &= !(1 << index % U32_bits);
        }

        previous
    }

    pub fn get(&self, index: usize) -> bool {
       if self.size / U32_bits < index / U32_bits {
            return false;
       }
        
       self.data[index / U32_bits] & (1 << index % U32_bits) != 0
    }
}
