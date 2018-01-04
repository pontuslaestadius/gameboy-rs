use share::*;
/// This file holds all executions of all opcode.


impl Session {

    /// Executes a given instruction on the session.
    /// Reading material for what each opcode will do:
    /// http://z80-heaven.wikidot.com/opcode-reference-chart
    pub fn execute(&mut self, instruction: Instruction) -> Result<(), Instruction> {

        let formatted_opcode: String = format!("{:?}", instruction.opcode); // TODO remove.

        let copy = self.registers.mem;

        match instruction.opcode {
            Opcode::NOP             => (), // The nop instruction is only to waste time.
            Opcode::LD(dt1, dt2)    => self.registers.ld(dt1.get(), dt2.get()),
            Opcode::INC(r)          => self.registers.inc(r),
            Opcode::DEC(r)          => self.registers.dec(r),
            Opcode::RST(imm8)       => self.registers.rst(imm8),

            // Alu is one instruction, but has two different input forms.
            Opcode::ALU(a, b)       => self.registers.alu(a, b),
            Opcode::ALU_(a, dt)     => self.registers.alu(a, dt.get()),

            // Anything invalid gets sent upstream.
            _ => return Err(instruction),
        }

        // Print the register if they changed.
        let new_copy = self.registers.mem;
        if copy == new_copy {
            println!("{:?}", new_copy);
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
    }

    /// Increments a register. Does nothing if already at max.
    pub fn inc(&mut self, register: u8) {
        let value = self.mem[register as usize];

        // If it already is at max value, we can't increment.
        if value != 255 {
            self.mem[register as usize] += 1;
        }
    }

    /// Decrements a register. Does nothing if already at max.
    pub fn dec(&mut self, register: u8) {
        let value = self.mem[register as usize];

        // If it already is at max value, we can't increment.
        if value != 0 {
            self.mem[register as usize] -= 1;
        }
    }

    // TODO this is experimental, and may not be an accurate implementation.
    /// http://z80-heaven.wikidot.com/instructions-set:rst
    /// The current PC value plus three is pushed onto the stack.
    /// The MSB is loaded with $00 and the LSB is loaded with imm8.
    pub fn rst(&mut self, imm8: u8) {
        self.mem[0] = 0;
        self.mem[7] = imm8;
        self.stack.push(self.pc as u16 +3);
    }

    pub fn alu(&mut self, operation: u8, input: u8) {

        // TODO modify flags.
        // TODO add all operators.

        match operation {
            0 => self.add_a(input),

            2 => self.sub(input),
            /*1 => ADC A,
            3 => SBC A,
            4 => self.and(input),
            5 => XOR
            6 => OR
            7 => CP
            _ => panic!("Invalid alu operation nr: {}", operation),
            */
            _ => println!("Invalid Operation: {}", operation),
        };
    }

    pub fn and(&mut self, input: u8) {
        panic!("TODO and");
        // TODO C and N flags cleared, P/V is parity, rest are altered by definition.
    }

    pub fn xor(&mut self, input: u8) {
        panic!("TODO xor");
        // TODO http://z80-heaven.wikidot.com/instructions-set:xor
    }

    pub fn cp(&mut self, input: u8) {
        panic!("TODO cp");
        // TODO http://z80-heaven.wikidot.com/instructions-set:cp
    }

    pub fn sub(&mut self, input: u8) {
        if (self.mem[0] < input) {
            self.mem[0] = 0;
        } else {
            self.mem[0] -= input;
        }
        // TODO N flag set, P/V is overflow, rest modified by definition.
    }

    pub fn add_a(&mut self, register: u8) {
        if register > 7 {
            println!("register too big. {}", register);
            return;
        }

        let add: u8 = self.mem[register as usize];
        if self.mem[0] > 255 -add {
            self.mem[0] = 255;
        } else {
            self.mem[0] += self.mem[register as usize];
        }
        // TODO (8-bit) N flag is reset, P/V is interpreted as overflow.
        // TODO (16-bit) preserves the S, Z and P/V flags, and H is undefined.
    }


    pub fn set_flag(&mut self, flag: Flag, value: bool) {

        let flag_index = 6;
        let mut smarbinary: SmartBinary = SmartBinary::new(self.mem[flag_index]);

        let flag = match flag {
            Flag::sign          => smarbinary.zer = value,
            Flag::zero          => smarbinary.one = value,
            Flag::five          => smarbinary.two = value,
            Flag::half_carry    => smarbinary.thr = value,
            Flag::three         => smarbinary.fou = value,
            Flag::parity_or_overflow => smarbinary.fiv = value,
            Flag::subtract      => smarbinary.six = value,
            Flag::carry         => smarbinary.sev = value,
        };
        self.mem[flag_index] = smarbinary.as_u8();
    }

}