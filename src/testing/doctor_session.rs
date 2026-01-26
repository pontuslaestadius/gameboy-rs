use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;

use log::info;

use crate::args::Args;
use crate::cartridge::Headers;
use crate::cpu::{Cpu, CpuSnapshot};
use crate::input::DummyInput;
use crate::mmu::Memory;
use crate::utils::output_string_diff;
use crate::*;

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct DoctorSession {
    pub golden_log: BufReader<File>,
    pub current_line: usize,
    pub memory: Bus<DummyInput>,
    pub cpu: Cpu,
    pub headers: Headers,
    pub history: RingBufferDoctor,
}

impl DoctorSession {
    pub fn new(buffer: Vec<u8>, args: Args) -> Self {
        let headers = Headers::new(&buffer);
        let file = File::open(args.doctor.golden_log.unwrap()).unwrap();
        let reader = BufReader::new(file);
        Self {
            golden_log: reader,
            current_line: 1,
            memory: Bus::new(buffer),
            cpu: Cpu::new(),
            headers,
            history: RingBufferDoctor::new(),
        }
    }
}

const RING_BUFFER_LENGTH: usize = 5;

pub struct RingBufferDoctor {
    pub entries: [Option<RingBufferDoctorState>; RING_BUFFER_LENGTH],
    pub head: usize,
}

impl RingBufferDoctor {
    pub fn new() -> Self {
        // Initialize with None because the buffer is empty at start
        Self {
            entries: Default::default(),
            head: 0,
        }
    }

    pub fn push(&mut self, instruction: OpcodeInfo, state: CpuSnapshot, line: usize) {
        self.entries[self.head] = Some(RingBufferDoctorState {
            instruction,
            state,
            line,
        });
        // Wrap around using the modulo operator
        self.head = (self.head + 1) % RING_BUFFER_LENGTH;
    }

    /// Returns the history from oldest to newest
    pub fn get_history(&self) -> Vec<&RingBufferDoctorState> {
        let mut history = Vec::new();
        for i in 0..RING_BUFFER_LENGTH {
            // Start from head (oldest) and go around
            let idx = (self.head + i) % RING_BUFFER_LENGTH;
            if let Some(ref entry) = self.entries[idx] {
                history.push(entry);
            }
        }
        history
    }
}

pub struct RingBufferDoctorState {
    pub instruction: OpcodeInfo,
    pub state: CpuSnapshot,
    pub line: usize,
}

impl DoctorSession {
    pub fn on_empty_golden_log(&mut self) {
        // Force write to memory to flush the serial port.
        self.memory.write_byte(0xFF02, 0x81);
        // Print a new line to avoid overwriting the test results.
        println!("");
        println!("PASSED! All {} lines matched.", self.current_line);
        exit(0);
    }
    pub fn on_mismatch(&self, expected: CpuSnapshot, _received: CpuSnapshot) {
        println!("ERROR: Mismatch CPU state.");
        println!("");
        let len = self.history.get_history().len();
        for (i, entry) in self.history.get_history().iter().enumerate() {
            if i == len - 1 {
                println!(
                    "{: <10} Expected: {}",
                    entry.line,
                    expected.to_doctor_string()
                );
                println!(
                    "           Was:      {}",
                    output_string_diff(
                        &expected.to_doctor_string(),
                        &entry.state.to_doctor_string()
                    )
                );
            } else {
                println!(
                    "{: <10} State:    {}",
                    entry.line,
                    entry.state.to_doctor_string()
                );
                println!("           Instr:    {}", entry.instruction);
            }
        }

        exit(1);
    }

    pub fn next(&mut self) {
        let mut expected: String = String::new();
        let _ = self.golden_log.read_line(&mut expected);
        let expected = expected.trim_end();

        if expected.is_empty() {
            self.on_empty_golden_log();
        }
        let expected = CpuSnapshot::from_string(expected).unwrap();
        let received = self.cpu.take_snapshot(&self.memory);
        let (code, _nr) = self.cpu.get_current_opcode(&self.memory);
        info!("Line: {}", self.current_line);
        info!("Code: {}", code);
        info!("State: {}", received.to_doctor_string());
        self.history.push(code, received, self.current_line);
        self.cpu.step(&mut self.memory);

        if expected != received {
            self.on_mismatch(expected, received);
        }
        self.current_line += 1;
    }
}

pub fn doctor_main_loop(mut session: DoctorSession) {
    loop {
        session.next();
    }
}
