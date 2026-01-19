use std::process::exit;

use crate::Memory;
use crate::cartridge::Headers;
use crate::cpu::Cpu;

pub trait SessionHandler {
    fn next(&mut self) -> Result<(), String>;
}

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
}
impl SessionHandler for Session {
    fn next(&mut self) -> Result<(), String> {
        self.cpu.step(&mut self.memory);
        return Ok(());
    }
}

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct DoctorSession {
    pub instruction_count: usize,
    pub count: usize,
    pub memory: Memory,
    pub cpu: Cpu,
    pub headers: Headers,
}

impl DoctorSession {
    pub fn new(buffer: Vec<u8>, count: usize) -> Self {
        let headers = Headers::new(&buffer);
        Self {
            instruction_count: count,
            count: 0,
            memory: Memory::new(buffer),
            cpu: Cpu::new(),
            headers,
        }
    }
}

impl SessionHandler for DoctorSession {
    fn next(&mut self) -> Result<(), String> {
        self.cpu.step(&mut self.memory);
        self.count += 1;
        if self.count >= self.instruction_count {
            exit(0);
        }
        return Ok(());
    }
}

pub enum SessionType {
    Normal(Session),
    Doctor(DoctorSession),
}

// Implement the trait for the Enum itself
impl SessionHandler for SessionType {
    fn next(&mut self) -> Result<(), String> {
        match self {
            SessionType::Normal(s) => s.next()?,
            SessionType::Doctor(s) => s.next()?,
        }
        Ok(())
    }
}

pub fn select_session_impl(buffer: Vec<u8>, debug_doctor: Option<usize>) -> SessionType {
    if let Some(c) = debug_doctor {
        SessionType::Doctor(DoctorSession::new(buffer, c))
    } else {
        SessionType::Normal(Session::new(buffer))
    }
}
