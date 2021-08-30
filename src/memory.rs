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
