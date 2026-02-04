use std::path::Path;

use gameboy_rs::{
    cartridge::{self, Headers},
    cpu::Cpu,
    mmu::Bus,
};

use crate::common::{EvaluationSpec, RuntimeSession};

pub struct RuntimeBuilder<E: EvaluationSpec> {
    cpu: Option<Cpu>,
    rom_data: Option<Vec<u8>>,
    evaluator: E,
}

impl RuntimeBuilder<NoopEvaluator> {
    pub fn new() -> Self {
        Self {
            cpu: None,
            rom_data: None,
            evaluator: NoopEvaluator,
        }
    }
}

impl<E: EvaluationSpec> RuntimeBuilder<E> {
    /// Load the ROM buffer and parse headers
    pub fn with_rom_path(self, rom_path: &Path) -> Self {
        let buffer = cartridge::load_rom(rom_path).unwrap();
        self.with_rom_data(buffer)
    }

    /// Load the ROM buffer and parse headers
    pub fn with_rom_data(mut self, data: Vec<u8>) -> Self {
        self.rom_data = Some(data);
        self
    }

    /// Provide a custom CPU state (e.g., for specific test entry points)
    pub fn with_cpu(mut self, cpu: Cpu) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// Swap the current evaluator for a different one
    pub fn with_evaluator<NewE: EvaluationSpec>(self, eval: NewE) -> RuntimeBuilder<NewE> {
        RuntimeBuilder {
            cpu: self.cpu,
            rom_data: self.rom_data,
            evaluator: eval,
        }
    }

    pub fn build(self) -> RuntimeSession<E> {
        let rom = self
            .rom_data
            .expect("ROM data is required to build a session");
        let headers = Headers::new(&rom);
        let memory = Bus::new(rom);
        let cpu = self.cpu.unwrap_or_else(Cpu::new);

        RuntimeSession {
            cpu,
            memory,
            headers,
            evaluator: self.evaluator,
        }
    }
}

/// A fallback evaluator that does nothing.
/// Useful for standard emulation where no testing comparison is required.
pub struct NoopEvaluator;

impl EvaluationSpec for NoopEvaluator {}
