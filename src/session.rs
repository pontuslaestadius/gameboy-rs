use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;

use crate::Memory;
use crate::args::Args;
use crate::cartridge::Headers;
use crate::cpu::Cpu;
use crate::instruction::{OPCODES, OpcodeInfo};

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
    pub golden_log: BufReader<File>,
    pub current_line: usize,
    pub memory: Memory,
    pub cpu: Cpu,
    pub headers: Headers,
    pub previous_instruction: OpcodeInfo,
}

impl DoctorSession {
    pub fn new(buffer: Vec<u8>, args: Args) -> Self {
        let headers = Headers::new(&buffer);
        let file = File::open(args.doctor.golden_log.unwrap()).unwrap();
        let reader = BufReader::new(file);
        Self {
            golden_log: reader,
            current_line: 1,
            memory: Memory::new(buffer),
            cpu: Cpu::new(),
            headers,
            previous_instruction: OPCODES[0].unwrap(), // Maps out to NOP.
        }
    }
}

impl SessionHandler for DoctorSession {
    fn next(&mut self) -> Result<(), String> {
        let mut expected: String = String::new();
        let _ = self.golden_log.read_line(&mut expected);
        let expected = expected.trim_end();
        if expected.is_empty() {
            println!("PASSED! All {} lines matched.", self.current_line);
            exit(0);
        }
        let received: String = self.cpu.format_for_doctor(&self.memory);
        self.cpu.step(&mut self.memory);
        if expected != received {
            println!("{}|Doctor Diff!", self.current_line);
            println!("Expected: {}", expected);
            println!("Received: {}", received);
            exit(1);

            // TODO: perform some witch-craft to do equal or better than Doctor output.
            // Need to track the PREVIOUS opcode executed.
        }
        self.current_line += 1;
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

pub fn select_session_impl(buffer: Vec<u8>, args: Args) -> SessionType {
    #[cfg(feature = "doctor")]
    return SessionType::Doctor(DoctorSession::new(buffer, args));
    #[cfg(not(feature = "doctor"))]
    return SessionType::Normal(Session::new(buffer));
}
