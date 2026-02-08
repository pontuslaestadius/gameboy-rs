use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;

use gameboy_rs::cpu::{Cpu, CpuSnapshot};
use gameboy_rs::input::DummyInput;
use gameboy_rs::mmu::Bus;
use gameboy_rs::utils::output_string_diff;

use crate::common::ring_buffer_doctor::RingBufferDoctor;
use crate::common::{EvaluationSpec, dump_log, init_logger};

// pub enum EvaluationMode {
//     /// A 'doctor' test pairs a test rom with a line-by-line
//     /// CPU state comparison, offering deep insight.
//     GoldenLog,
//     /// Expect the test to print "Passed" to the memory serial bus.
//     SerialBus,
//     /// We've finished reading the file, it's the last strict option,
//     /// and should only be used if no other mode is available.
//     EndOfFile,
//     /// If we're running in interactive mode, the user has the absolute say.
//     /// This is equivelent to saying "no evaluation."
//     None,
// }

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct DoctorEvaluator {
    pub golden_log: BufReader<File>,
    pub current_line: usize,
    pub history: RingBufferDoctor,
    pub is_failure: Option<CpuSnapshot>,
}

impl EvaluationSpec for DoctorEvaluator {
    fn pre_step(&mut self, cpu: &Cpu, bus: &Bus<DummyInput>) -> bool {
        // self.is_failure = true;
        // return false;
        match self.next_golden_log() {
            Some(expected) => {
                let received = cpu.take_snapshot(&bus);
                let (code, _nr) = cpu.get_current_opcode(&bus);

                // Push to history BEFORE execution (Pre-execution state)
                self.history.push(code.clone(), received, self.current_line);

                if expected != received {
                    self.is_failure = Some(expected);
                    return false;
                }

                self.current_line += 1;
                true
            }
            None => false,
        }
    }

    fn report(&self, _cpu: &Cpu, memory: &Bus<DummyInput>) {
        if let Some(expected) = self.is_failure {
            // If this fails, we can look at history to see the state
            // immediately after the silent hijack but before this opcode.
            if let Some(entry) = self.history.last() {
                println!("{:?}", memory.ppu);
                self.on_mismatch(expected, entry.state);
                exit(1);
            }
        }
    }
}

impl DoctorEvaluator {
    pub fn new(golden_log: &str) -> Self {
        init_logger().unwrap();
        let file = File::open(golden_log).unwrap();
        let reader = BufReader::new(file);
        Self {
            golden_log: reader,
            current_line: 1,
            history: RingBufferDoctor::new(),
            is_failure: None,
        }
    }

    // pub fn on_empty_golden_log(&mut self) {
    //     // Force write to memory to flush the serial port.
    //     self.memory.write_byte(0xFF02, 0x81);
    //     // Print a new line to avoid overwriting the test results.
    //     println!();
    //     println!("PASSED! All {} lines matched.", self.current_line);
    //     exit(0);
    // }
    pub fn on_mismatch(&self, expected: CpuSnapshot, received: CpuSnapshot) {
        dump_log();
        println!("--- Mismatch CPU state ------------------------------");
        let len = self.history.get_history().len();
        for (i, entry) in self.history.get_history().iter().enumerate() {
            if i == len - 1 {
                // Print as Hex to save chars.
                println!(
                    "{:06X} Expected: {}",
                    entry.line,
                    expected.to_doctor_string()
                );
                println!(
                    "       Was:      {}",
                    output_string_diff(
                        &expected.to_doctor_string(),
                        &received.to_doctor_string(),
                        // &entry.state.to_doctor_string()
                    )
                );
            } else {
                println!(
                    "{:06X} State:    {}",
                    entry.line,
                    entry.state.to_doctor_string()
                );
            }
            println!("       Instr:    {}", entry.instruction);
        }
    }

    pub fn next_golden_log(&mut self) -> Option<CpuSnapshot> {
        let mut expected: String = String::new();
        let _ = self.golden_log.read_line(&mut expected);
        let expected = expected.trim_end();

        if expected.is_empty() {
            return None;
        }
        // Unwrap is allowed here, since it's test data, it either works,
        // Or we go new game plus.
        let expected = CpuSnapshot::from_string(expected).unwrap();
        Some(expected)
    }
}
