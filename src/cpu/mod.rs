pub mod immediate;
pub mod instruction_set;

pub mod operand;
pub mod register;
use crate::instruction::*;
use crate::*;

#[derive(Debug)]
pub struct Cpu {
    // 8-bit Registers
    pub a: u8,
    pub f: u8, // Flags Register
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,

    // 16-bit Special Registers
    pub pc: u16, // Program Counter
    pub sp: u16, // Stack Pointer

    // Internal state
    pub halted: bool,

    pub ime: bool, // Interrupt Master Enable (The "Master Switch")

    // Use an enum or a counter for the EI delay.
    // Since it's exactly one instruction, a simple 0, 1, 2 counter works well.
    pub ime_scheduled: u8,

    halt_bug_triggered: bool,
    // These represent the hardware registers at 0xFF0F and 0xFFFF
    pub if_reg: u8, // Interrupt Flag (What happened?)
    pub ie_reg: u8, // Interrupt Enable (What do we care about?)
}

struct AluResult {
    value: u8,
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            // These values are standard for the GB after the boot ROM runs
            a: 0x01,
            f: 0xB0, // Flags
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            pc: 0x0100, // Entry point for cartridges
            sp: 0xFFFE,
            halted: false,
            ime: false,
            ime_scheduled: 0,
            if_reg: 0,
            ie_reg: 0,
            halt_bug_triggered: false,
        }
    }
    pub fn reset_post_boot(&mut self) {
        self.a = 0x01;
        // F = 0xB0 -> Z:1, N:0, H:1, C:1 (Upper nibble is 1011)
        self.f = 0xB0;
        self.b = 0x00;
        self.c = 0x13;
        self.d = 0x00;
        self.e = 0xD8;
        self.h = 0x01;
        self.l = 0x4D;
        self.sp = 0xFFFE;
        self.pc = 0x0100; // The standard entry point for all cartridges
    }
    fn service_interrupt(&mut self, bit: u8, bus: &mut impl memory_trait::Memory) {
        // 1. Disable interrupts to prevent recursive nesting
        self.ime = false;

        // 2. Acknowledge the interrupt by clearing the specific bit in IF
        // Note: Use bitmask clearing, not XOR, to be safe.
        self.if_reg &= !(1 << bit);

        // 3. Push current PC onto the stack
        // The stack grows downwards, so we decrement SP before each write.
        let pc_high = (self.pc >> 8) as u8;
        let pc_low = (self.pc & 0xFF) as u8;

        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, pc_high);

        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, pc_low);

        // 4. Jump to the vector address
        // Priority: V-Blank (0x40), LCD (0x48), Timer (0x50), Serial (0x58), Joypad (0x60)
        self.pc = match bit {
            0 => 0x0040,  // V-Blank
            1 => 0x0048,  // LCD STAT
            2 => 0x0050,  // Timer
            3 => 0x0058,  // Serial
            4 => 0x0060,  // Joypad
            _ => self.pc, // Should never happen
        };
    }

    fn check_interrupts(&mut self, bus: &mut impl memory_trait::Memory) {
        let if_reg = bus.read(0xFF0F);
        let ie_reg = bus.read(0xFFFF);
        let pending = if_reg & ie_reg;

        if pending == 0 {
            return;
        }

        // ANY pending interrupt wakes the CPU, even if IME is 0
        if self.halted {
            self.halted = false;
        }

        // Only jump to the handler if Master Enable is ON
        if self.ime {
            for bit in 0..5 {
                if (pending >> bit) & 1 == 1 {
                    // Clear the flag and push PC
                    let cleared_if = if_reg & !(1 << bit);
                    bus.write(0xFF0F, cleared_if);
                    self.service_interrupt(bit, bus);
                    break;
                }
            }
        }
    }
    fn apply_flags(&mut self, spec: &FlagSpec, res: InstructionResult) {
        self.update_single_flag(FLAG_Z, spec.z, res.z);
        self.update_single_flag(FLAG_N, spec.n, res.n); // N is almost always hardcoded in Spec
        self.update_single_flag(FLAG_H, spec.h, res.h);
        self.update_single_flag(FLAG_C, spec.c, res.c);
    }

    fn update_single_flag(&mut self, bit: u8, action: FlagAction, proposed: bool) {
        match action {
            FlagAction::Calculate => self.set_flag(bit, proposed),
            FlagAction::Set => self.set_flag(bit, true),
            FlagAction::Reset => self.set_flag(bit, false),
            FlagAction::Invert => {
                let current = self.get_flag(bit);
                self.set_flag(bit, !current);
            }
            FlagAction::None => {} // Instruction doesn't touch this flag. Keep old value.
        }
    }
    fn alu_8bit_add(&self, a: u8, b: u8, use_carry: bool) -> AluResult {
        let c_in = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };

        let res = (a as u16) + (b as u16) + (c_in as u16);
        let res_u8 = res as u8;

        // Half-Carry: Carry out of bit 3 into bit 4
        // We check if the sum of the lower nibbles exceeds 0xF
        let h_bit = (a & 0x0F) + (b & 0x0F) + c_in > 0x0F;

        AluResult {
            value: res_u8,
            z: res_u8 == 0,
            n: false,
            h: h_bit,
            c: res > 0xFF,
        }
    }

    fn alu_8bit_sub(&self, a: u8, b: u8, use_carry: bool) -> AluResult {
        let c_in = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };

        // Standard subtraction result
        let res = (a as i16) - (b as i16) - (c_in as i16);
        let res_u8 = res as u8;

        // Half-Carry (Half-Borrow): Set if there is no borrow from bit 4.
        // In GB terms: bit 3 of 'a' was less than (bit 3 of 'b' + c_in)
        let h_bit = (a & 0x0F) < (b & 0x0F) + c_in;

        // Carry (Borrow): Set if the result is negative (a borrow from bit 8)
        let c_bit = (a as u16) < (b as u16) + (c_in as u16);

        AluResult {
            value: res_u8,
            z: res_u8 == 0,
            n: true,
            h: h_bit,
            c: c_bit,
        }
    }

    pub fn set_c(&mut self, value: bool) {
        self.set_flag(FLAG_C, value);
    }

    pub fn set_h(&mut self, value: bool) {
        self.set_flag(FLAG_H, value);
    }

    pub fn set_n(&mut self, value: bool) {
        self.set_flag(FLAG_N, value);
    }
    pub fn set_z(&mut self, value: bool) {
        self.set_flag(FLAG_Z, value);
    }

    pub fn get_flag(&self, flag: u8) -> bool {
        (self.f & flag) != 0
    }

    pub fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.f |= flag;
        } else {
            self.f &= !flag;
        }
    }

    pub fn request_ime(&mut self) {
        self.ime_scheduled = 2;
    }

    pub fn step(&mut self, bus: &mut impl memory_trait::Memory) {
        // 1. Check for interrupts (WAKE UP LOGIC)
        let pending = bus.read(0xFF0F) & bus.read(0xFFFF);

        if pending != 0 {
            self.halted = false; // Any pending interrupt wakes the CPU
            if self.ime {
                self.service_interrupt(0, bus); // Jump to vector
                return; // Skip normal instruction fetch
            }
        }

        // 2. If still halted, just tick cycles and return
        if self.halted {
            bus.increment_cycles(4); // HALT consumes 4 cycles per "step"
            return;
        }

        // 2. Log State (Doctor expects state BEFORE the instruction)
        info!("{}", self.format_for_doctor(bus));

        // This is the point where the CPU decides: "Do I execute the next instruction
        // at PC, or do I hijack the PC and go to a vector?"
        self.check_interrupts(bus);

        // 3. IME Delay Logic
        // If EI was called in the previous step, IME turns on NOW.
        // Note: check_interrupts already ran, so it won't fire until the NEXT step.
        if self.ime_scheduled > 0 {
            self.ime_scheduled -= 1;
            if self.ime_scheduled == 0 {
                self.ime = true;
            }
        }

        let opcode = bus.read(self.pc);

        if self.halt_bug_triggered {
            // The PC DOES NOT increment this time.
            // The next instruction will read this same byte again.
            self.halt_bug_triggered = false;
        } else {
            self.pc = self.pc.wrapping_add(1);
        }

        let op = if opcode == CB_PREFIX_OPCODE_BYTE {
            let cb = bus.read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            CB_OPCODES[cb as usize]
        } else {
            OPCODES[opcode as usize]
        };

        if let Some(code) = op {
            let result = self.dispatch(code, bus);
            self.apply_flags(&code.flags, result);
            bus.increment_cycles(result.cycles as u64);
        }
    }

    fn calculate_dec_8bit(&self, value: u8) -> (u8, bool, bool, bool) {
        let res = value.wrapping_sub(1);

        // Flags:
        let z = res == 0;
        let n = true; // Always true for DEC
        // Half-Carry: Set if there was a borrow from bit 4
        // (i.e., the lower nibble was 0x0 before the decrement)
        let h = (value & 0x0F) == 0;

        (res, z, n, h)
    }
    fn check_condition(&self, target: Target) -> bool {
        match target {
            Target::Condition(cond) => match cond {
                Condition::NotZero => (self.f & FLAG_Z) == 0,
                Condition::Zero => (self.f & FLAG_Z) != 0,
                Condition::NotCarry => (self.f & FLAG_C) == 0,
                Condition::Carry => (self.f & FLAG_C) != 0,
                _ => true, // Always true for unconditional
            },
            _ => true, // Not a conditional target
        }
    }

    fn get_src_val(&mut self, instruction: &OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        if instruction.operands.len() == 1 {
            // Single operand instructions (like INC C or POP BC)
            let (target, _) = instruction.operands[0];
            self.read_target(target, bus).as_u8()
        } else {
            // Two operand instructions (like CP A, n8 or OR A, C)
            // Usually, the second one (index 1) is the "Source"
            let (target, _) = instruction.operands[1];
            self.read_target(target, bus).as_u8()
        }
    }
    /// Reads the actual value for a given operand target.
    /// This may increment PC if it reads immediate values from memory.
    fn read_target(&mut self, target: Target, bus: &mut impl memory_trait::Memory) -> OperandValue {
        match target {
            Target::Register8(reg) => OperandValue::U8(self.get_reg8(reg)),

            Target::Register16(reg) => OperandValue::U16(self.get_reg16(reg)),

            Target::Immediate8 => {
                let val = bus.read(self.pc);
                self.pc = self.pc.wrapping_add(1);
                OperandValue::U8(val)
            }

            Target::Immediate16 => {
                let val = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                OperandValue::U16(val)
            }

            // Memory access: (HL), (BC), (DE)
            Target::AddrRegister16(reg) => {
                let addr = self.get_reg16(reg);
                OperandValue::U8(bus.read(addr))
            }
            Target::AddrRegister8(_) => todo!(),

            // LDH (a8) - High RAM access (0xFF00 + immediate byte)
            Target::AddrImmediate8 => {
                let offset = bus.read(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                OperandValue::U8(bus.read(0xFF00 | offset))
            }

            // (nn) - 16-bit address read
            Target::AddrImmediate16 => {
                let addr = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                OperandValue::U8(bus.read(addr))
            }
            // 1. Indirect Read with Side Effects (e.g., LD A, (HL+))
            Target::AddrRegister16Increment(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read(addr);
                self.set_reg16(reg, addr.wrapping_add(1)); // Increment side effect
                OperandValue::U8(val)
            }
            Target::AddrRegister16Decrement(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read(addr);
                self.set_reg16(reg, addr.wrapping_sub(1)); // Decrement side effect
                OperandValue::U8(val)
            }

            // 2. Relative Offset (JR instructions)
            Target::Relative8 => {
                let val = bus.read(self.pc) as i8; // Cast to signed immediately
                self.pc = self.pc.wrapping_add(1);
                OperandValue::I8(val) // You need an I8 variant in OperandValue
            }

            // 3. Conditions (Check flags)
            Target::Condition(cond) => {
                let met = match cond {
                    Condition::NotZero => !self.get_flag(FLAG_Z),
                    Condition::Zero => self.get_flag(FLAG_Z),
                    Condition::NotCarry => !self.get_flag(FLAG_C),
                    Condition::Carry => self.get_flag(FLAG_C),
                };
                OperandValue::Bool(met) // You need a Bool variant in OperandValue
            }

            // 4. RST Vectors
            Target::Vector(v) => OperandValue::U16(v as u16),

            Target::StackPointer => OperandValue::U16(self.sp),

            Target::Bit(b) => OperandValue::U8(b),
        }
    }

    pub fn write_target(
        &mut self,
        target: Target,
        value: OperandValue,
        mmu: &mut impl memory_trait::Memory,
    ) {
        match (target, value) {
            (Target::Register8(reg), OperandValue::U8(v)) => self.set_reg8(reg, v),
            (Target::Register16(reg), OperandValue::U16(v)) => self.set_reg16(reg, v),
            (Target::StackPointer, OperandValue::U16(v)) => self.sp = v,
            // Matches (HL), (BC), or (DE)
            // a16 is a common write target (e.g., LD (a16), SP)
            (Target::AddrRegister16(reg), OperandValue::U8(v)) => {
                let addr = self.get_reg16(reg);
                mmu.write(addr, v);
            }

            (Target::AddrRegister16Decrement(reg), OperandValue::U8(v)) => {
                let addr = self.get_reg16(reg);
                mmu.write(addr, v);

                // The side effect: decrement the pointer
                let new_val = addr.wrapping_sub(1);
                self.set_reg16(reg, new_val);
            }
            (Target::AddrRegister16Increment(reg), OperandValue::U8(v)) => {
                let addr = self.get_reg16(reg);
                mmu.write(addr, v);

                // The side effect: increment the pointer
                let new_val = addr.wrapping_add(1);
                self.set_reg16(reg, new_val);
            }

            (Target::AddrImmediate16, value) => {
                // Read the 16-bit address (LSB first)
                let low = mmu.read(self.pc) as u16;
                let high = mmu.read(self.pc.wrapping_add(1)) as u16;
                let addr = (high << 8) | low;
                self.pc = self.pc.wrapping_add(2);

                match value {
                    OperandValue::U8(v) => mmu.write(addr, v),
                    OperandValue::U16(v) => {
                        // e.g., LD (a16), SP writes 16 bits
                        mmu.write(addr, (v & 0xFF) as u8);
                        mmu.write(addr.wrapping_add(1), (v >> 8) as u8);
                    }
                    _ => todo!(),
                }
            }

            (Target::AddrImmediate8, v) => {
                // 1. Read the 8-bit offset following the opcode
                let offset = mmu.read(self.pc);
                self.pc = self.pc.wrapping_add(1);

                // 2. Construct the High RAM address
                let addr = 0xFF00 | (offset as u16);

                // 3. Write the 8-bit value to that address
                mmu.write(addr, v.as_u8());
            }
            _ => panic!(
                "write_target: Invalid write target or value mismatch, {:?}, {:?}",
                target, value
            ),
        }
    }

    pub fn get_reg16(&self, reg: Reg16) -> u16 {
        match reg {
            Reg16::BC => u16::from_be_bytes([self.b, self.c]),
            Reg16::DE => u16::from_be_bytes([self.d, self.e]),
            Reg16::HL => u16::from_be_bytes([self.h, self.l]),
            Reg16::SP => self.sp,
            Reg16::AF => u16::from_be_bytes([self.a, self.f]),
            _ => panic!("Cannot get PC reg."),
        }
    }

    pub fn set_reg16(&mut self, reg: Reg16, val: u16) {
        let bytes = val.to_be_bytes();
        match reg {
            Reg16::BC => {
                self.b = bytes[0];
                self.c = bytes[1];
            }
            Reg16::DE => {
                self.d = bytes[0];
                self.e = bytes[1];
            }
            Reg16::HL => {
                self.h = bytes[0];
                self.l = bytes[1];
            }
            Reg16::SP => self.sp = val,
            Reg16::AF => {
                self.a = bytes[0];
                // Note: The lower 4 bits of the F register are always 0 on Game Boy
                self.f = bytes[1] & 0xF0;
            }
            _ => panic!("Cannot get PC reg."),
        }
    }

    pub fn get_reg8(&self, reg: Reg8) -> u8 {
        match reg {
            Reg8::A => self.a,
            Reg8::B => self.b,
            Reg8::C => self.c,
            Reg8::D => self.d,
            Reg8::E => self.e,
            Reg8::H => self.h,
            Reg8::L => self.l,
        }
    }

    pub fn set_reg8(&mut self, reg: Reg8, val: u8) {
        match reg {
            Reg8::A => self.a = val,
            Reg8::B => self.b = val,
            Reg8::C => self.c = val,
            Reg8::D => self.d = val,
            Reg8::E => self.e = val,
            Reg8::H => self.h = val,
            Reg8::L => self.l = val,
        }
    }

    /// Reads a 16-bit value from the current Stack Pointer and increments SP by 2.
    /// Little-Endian: The byte at SP is the low byte, SP+1 is the high byte.
    pub fn pop_u16(&mut self, bus: &impl memory_trait::Memory) -> u16 {
        let low = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        let high = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        (high << 8) | low
    }

    /// Decrements SP by 2 and writes a 16-bit value to the stack.
    /// Little-Endian: The high byte goes to SP-1, the low byte goes to SP-2.
    pub fn push_u16(&mut self, bus: &mut impl memory_trait::Memory, val: u16) {
        let high = (val >> 8) as u8;
        let low = (val & 0xFF) as u8;

        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, high);

        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, low);
    }

    // Helper for the A-versions
    // fn set_flags_rotate(&mut self, res: u8, carry: bool, is_a_version: bool) {
    //     self.set_flag(FLAG_Z, if is_a_version { false } else { res == 0 });
    //     self.set_flag(FLAG_N, false);
    //     self.set_flag(FLAG_H, false);
    //     self.set_flag(FLAG_C, carry);
    // }

    fn get_reg16_from_target(&self, target: Target) -> u16 {
        match target {
            Target::Register16(reg) => self.get_reg16(reg),
            Target::StackPointer => self.sp,
            // Add any other 16-bit targets your build.rs might generate
            _ => panic!("Target {:?} is not a 16-bit register", target),
        }
    }
    fn set_reg16_from_target(&mut self, target: Target, value: u16) {
        match target {
            Target::Register16(reg) => self.set_reg16(reg, value),
            Target::StackPointer => self.sp = value,
            _ => panic!("Target {:?} is not a 16-bit register", target),
        }
    }
    pub fn format_for_doctor(&self, bus: &impl memory_trait::Memory) -> String {
        // Read 4 bytes starting at PC for the PCMEM section
        let pcmem0 = bus.read(self.pc);
        let pcmem1 = bus.read(self.pc.wrapping_add(1));
        let pcmem2 = bus.read(self.pc.wrapping_add(2));
        let pcmem3 = bus.read(self.pc.wrapping_add(3));

        // Format: A:00 F:11 B:22 C:33 D:44 E:55 H:66 L:77 SP:8888 PC:9999 PCMEM:AA,BB,CC,DD
        format!(
            "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
            self.a,
            self.f,
            self.b,
            self.c,
            self.d,
            self.e,
            self.h,
            self.l,
            self.sp,
            self.pc,
            pcmem0,
            pcmem1,
            pcmem2,
            pcmem3
        )
    }
}

use std::fmt;

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format Flags: [ZNHC] (uppercase if set, lowercase/dash if clear)
        let z = if self.get_flag(FLAG_Z) { 'Z' } else { '-' };
        let n = if self.get_flag(FLAG_N) { 'N' } else { '-' };
        let h = if self.get_flag(FLAG_H) { 'H' } else { '-' };
        let c = if self.get_flag(FLAG_C) { 'C' } else { '-' };

        write!(
            f,
            "A:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} Flags:[{}{}{}{}]",
            self.get_reg8(Reg8::A),
            self.get_reg8(Reg8::B),
            self.get_reg8(Reg8::C),
            self.get_reg8(Reg8::D),
            self.get_reg8(Reg8::E),
            self.get_reg8(Reg8::H),
            self.get_reg8(Reg8::L),
            self.sp,
            z,
            n,
            h,
            c
        )
    }
}
