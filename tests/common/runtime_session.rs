use gameboy_rs::{cartridge::Headers, cpu::Cpu, input::DummyInput, mmu::Bus, mmu::Memory};

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct RuntimeSession<E: EvaluationSpec> {
    pub cpu: Cpu,
    pub memory: Bus<DummyInput>,
    pub headers: Headers,
    pub evaluator: E, // Use generics instead of 'dyn' for better performance
}

pub trait EvaluationSpec {
    /// Called after every CPU step.
    /// Returns true to continue, false to stop (e.g., mismatch or success).
    fn evaluate(&mut self, _cpu: &Cpu, _memory: &Bus<DummyInput>) -> bool {
        true
    }

    /// Called after interrupt handling, but before instruction processing.

    fn pre_step(&mut self, _cpu: &Cpu, _memory: &Bus<DummyInput>) -> bool {
        true
    }

    /// Called when the session ends to report findings.
    fn report(&self, _cpu: &Cpu, _memory: &Bus<DummyInput>) {}

    /// Called when the cpu is hijacked/halted by interrupts.
    // If halted and no wake-up, the evaluator decides
    // if we wait or break (usually return true to keep waiting)
    fn on_interrupt(&mut self, _cpu: &Cpu, _bus: &Bus<DummyInput>) -> bool {
        true
    }
}

impl<E: EvaluationSpec> RuntimeSession<E> {
    pub fn run_to_completition(&mut self) {
        loop {
            if !self.step() {
                break;
            }
        }
        self.evaluator.report(&self.cpu, &self.memory);
    }

    pub fn step(&mut self) -> bool {
        // 1. Hardware Phase: Process hijacks/halts between instructions.
        while (self.memory.pending_interrupt() && self.cpu.ime) || self.cpu.halted {
            let cycles = self.cpu.step(&mut self.memory);
            self.memory.tick_components(cycles);

            if !self.evaluator.on_interrupt(&self.cpu, &self.memory) {
                return false;
            }

            if self.cpu.halted && !self.memory.pending_interrupt() {
                // Return true here if we want to keep waiting for an interrupt
                return true;
            }
        }

        // 2. Pre-Instruction Phase: Snapshot for logging (Matches Doctor format)
        if !self.evaluator.pre_step(&self.cpu, &self.memory) {
            return false;
        }

        // 3. Instruction Phase: Execute the actual opcode
        let cycles = self.cpu.step(&mut self.memory);
        self.memory.tick_components(cycles);

        // 4. Post-Instruction Phase: Final checks
        self.evaluator.evaluate(&self.cpu, &self.memory)
    }
}
