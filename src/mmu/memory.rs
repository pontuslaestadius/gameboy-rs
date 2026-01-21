use crate::memory_trait;

/// 64 Kb - The standard Game Boy address space
const MEMORY_SIZE: usize = 1024 * 64;

pub struct Bus {
    // Must use a Vec since an Array would use the stack, and crash the application.
    // Using the heap is required.
    pub rom_size: usize,
    // This puts exactly 64KB on the HEAP, not the STACK
    pub data: Box<[u8; MEMORY_SIZE]>,
    total_cycles: u64,
}

impl Bus {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let rom_size = rom_data.len();
        // Create a zeroed array on the heap
        let mut buffer = Box::new([0u8; MEMORY_SIZE]);

        // Copy ROM data into the beginning
        let copy_len = std::cmp::min(rom_size, MEMORY_SIZE);
        buffer[..copy_len].copy_from_slice(&rom_data[..copy_len]);

        Bus {
            rom_size,
            data: buffer,
            total_cycles: 0,
        }
    }
}

impl memory_trait::Memory for Bus {
    // fn read(&self, addr: u16) -> u8 {
    fn read(&self, addr: u16) -> u8 {
        if addr == 0xFF44 {
            // If we are in the middle of a CPU test, just return 0x90
            // to let the CPU pass the 'Wait for V-Blank' loop.
            //         // Return a rotating value to satisfy "Wait for LY == X" loops
            //         // This is a common hack for CPU-only testing
            // return ((self.total_cycles) / 456 % 154) as u8;
            return 0x90;
        }
        self.data[addr as usize]
    }
    fn increment_cycles(&mut self, value: u64) {
        // Optional: Stop after a few million cycles if you're running headless
        if self.total_cycles > 100_000_000_000 {
            panic!("Test suite: Too many cycles: {} > 10b", self.total_cycles);
        }
        self.total_cycles += value
    }
    fn write(&mut self, addr: u16, val: u8) {
        // 1. Handle Echo RAM Mirroring (0xE000 - 0xFDFF mirrors 0xC000 - 0xDFFF)
        if (0xE000..=0xFDFF).contains(&addr) {
            let mirrored_addr = addr - 0x2000;
            self.data[mirrored_addr as usize] = val;
        }

        if addr == 0xFF44 {
            // LY is read-only on real hardware.
            // Writing to it usually resets it to 0,
            // but for this hack, we just ignore the write.
        }

        // 2. Handle Serial/LY/other hooks here...
        // Hooks into Serial Port Link Cable interface.
        if addr == 0xFF02 && val == 0x81 {
            let c = self.read(0xFF01) as char;
            print!("{}", c); // This prints test results like "CPU PASS"
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
        self.data[addr as usize] = val;
    }
}
