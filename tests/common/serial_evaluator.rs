use std::process::exit;

use gameboy_rs::{cpu::Cpu, input::DummyInput, mmu::Bus};

use crate::common::EvaluationSpec;

pub struct SerialEvaluator {
    max_cycles: u64,
    cycles: u64,
}

impl SerialEvaluator {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            // For debug: this may take ~10s.
            // For release: ~0.7s before timeout.
            max_cycles: 10_000_000,
        }
    }
}

impl EvaluationSpec for SerialEvaluator {
    fn evaluate(&mut self, _cpu: &Cpu, bus: &Bus<DummyInput>) -> bool {
        self.cycles += 1;
        // We only check for success/fail every few thousand cycles
        // to avoid expensive string searching on every single opcode.
        if self.cycles % 10_000 == 0 {
            let output = String::from_utf8_lossy(&bus.serial_buffer);
            if output.contains("Passed") {
                return false;
            }
            if output.contains("Failed") {
                return false;
            }
        }
        self.cycles < self.max_cycles
    }

    fn report(&self, _cpu: &Cpu, bus: &Bus<DummyInput>) {
        let output = String::from_utf8_lossy(&bus.serial_buffer);
        if !output.contains("Passed") {
            if !output.is_empty() {
                println!("output: {}", output);
            }
            exit(1);
        }
        exit(0);
    }
}
