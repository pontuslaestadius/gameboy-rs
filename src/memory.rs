use crate::memory_trait;

/// 32 MB
const MEMORY_SIZE: usize = 1024 * 1024 * 32;

pub struct Memory {
    // Must use a Vec since an Array would use the stack, and crash the application.
    // Using the heap is required.
    pub rom_size: usize,
    pub data: Vec<u8>,
}

impl Memory {
    pub fn new(mut data: Vec<u8>) -> Self {
        let rom_size = data.len();
        data.resize(MEMORY_SIZE, 0);
        Memory { rom_size, data }
    }
}

impl memory_trait::Memory for Memory {
    fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }
}
