use share::*;
/// This file holds all executions of all opcode.



impl Session {
    pub fn execute(&mut self, instruction: Instruction) -> Result<(), Instruction> {

        let formatted_opcode: String = format!("{:?}", instruction.opcode); // TODO remove.

        match instruction.opcode {

            Opcode::LD(dt1, dt2)  => {
                self.registers.ld(dt1.get(), dt2.get());
            }

            Opcode::INC(r) => {
                self.registers.inc(r);
            }

            // Loops for invalid opcodes and stores them in the log file.
            Opcode::INVALID(_) => {
                return Err(instruction);
            }

            _ => println!("{}", formatted_opcode) // TODO replace with execution.
        }

        // TODO
        Ok(())
    }
}

impl Registers {

    /// Sets a register with a new u8 value.
    pub fn ld(&mut self, to: u8, from: u8) {

        let value = self.mem[from as usize];
        self.mem[to as usize] = value;

        println!("{:?}", self);
    }

    /// Increments a register. Does nothing if already at max.
    pub fn inc(&mut self, register: u8) {
        let value = self.mem[register as usize];

        // If it already is at max value, we can't increment.
        if value != 255 {
            self.mem[register as usize] += 1;
        }
    }
}