use log::info;

use crate::Memory;
use crate::cartridge::Headers;
use crate::cpu::Cpu;

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct Session {
    pub memory: Memory,
    pub cpu: Cpu,
    pub headers: Headers,
}

impl Session {
    pub fn new(buffer: Vec<u8>) -> Self {
        let headers = Headers::new(&buffer);
        // info!("Cartridge headers: {:?}", headers);
        Session {
            memory: Memory::new(buffer),
            cpu: Cpu::new(),
            headers,
        }
    }

    pub fn next(&mut self) -> Result<(), String> {
        self.cpu.step(&mut self.memory);
        return Ok(());
    }
}
