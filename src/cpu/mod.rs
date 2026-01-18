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
    pub ime: bool, // Interrupt Master Enable
    // Internal state, use to track EIA
    // The interrupts are not enabled until the instruction after the EI instruction.
    pub ime_requested: bool,
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
            ime_requested: false,
        }
    }
    fn apply_alu_flags(&mut self, res: AluResult) {
        self.set_flag(FLAG_Z, res.z);
        self.set_flag(FLAG_N, res.n);
        self.set_flag(FLAG_H, res.h);
        self.set_flag(FLAG_C, res.c);
    }
    fn alu_8bit_add(&self, a: u8, b: u8, use_carry: bool) -> AluResult {
        let carry_val = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };

        // Use u16 to detect the 8-bit Carry (result > 0xFF)
        let res = (a as u16) + (b as u16) + (carry_val as u16);
        let res_u8 = res as u8;

        // Half-Carry: (a_lower + b_lower + carry) > 0xF
        let h_bit = (a & 0x0F) + (b & 0x0F) + carry_val > 0x0F;

        AluResult {
            value: res_u8,
            z: res_u8 == 0,
            n: false, // Always false for ADD
            h: h_bit,
            c: res > 0xFF,
        }
    }
    fn alu_8bit_sub(&self, a: u8, b: u8, use_carry: bool) -> AluResult {
        let carry_val = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };

        // Use u16 for result to detect Carry easily
        let res = (a as u16)
            .wrapping_sub(b as u16)
            .wrapping_sub(carry_val as u16);
        let res_u8 = res as u8;

        // Half-Carry for subtraction: borrow from bit 4
        // Logic: ((a & 0xF) as i16) - ((b & 0xF) as i16) - (carry_val as i16) < 0
        let h_bit = (a & 0x0F) < (b & 0x0F) + carry_val;

        AluResult {
            value: res_u8,
            z: res_u8 == 0,
            n: true, // Always true for SUB/CP
            h: h_bit,
            c: res > 0xFF, // In wrapping_sub, a result > 0xFF indicates a borrow occurred
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

    pub fn step(&mut self, bus: &mut impl memory_trait::Memory) {
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        let op = if opcode == CB_PREFIX_OPCODE_BYTE {
            let cb = bus.read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            CB_OPCODES[cb as usize]
        } else {
            OPCODES[opcode as usize]
        };

        if let Some(code) = op {
            // Pass the bus into your execution logic

            info!("CPU: {}", self);
            // Since we increment PC before this, we decrement it in our log.
            info!("{:#X}. {:#X}. {}", self.pc - 1, opcode, code);
            let cycles = self.dispatch(code, bus);
            std::thread::sleep(T_CYCLE * cycles as u32);
            bus.increment_cycles(cycles as u64);
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

    // fn check_condition(&self, condition: Condition) -> bool {
    //     match condition {
    //         Condition::None => true, // Unconditional jump
    //         Condition::Z => self.get_flag(Z_FLAG),
    //         Condition::NZ => !self.get_flag(Z_FLAG),
    //         Condition::C => self.get_flag(C_FLAG),
    //         Condition::NC => !self.get_flag(C_FLAG),
    //     }
    // }

    /// Pushes a 16-bit value onto the stack
    fn push_u16(&mut self, bus: &mut impl memory_trait::Memory, value: u16) {
        let bytes = value.to_be_bytes(); // PC is stored High then Low
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, bytes[0]); // High byte
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, bytes[1]); // Low byte
    }

    // fn check_condition_operand(&self, target: Target) -> bool {
    //     match target {
    //         Target::Register8(Reg8::Z) => self.get_flag(FLAG_Z),
    //         Target::Register8(Reg8::NZ) => !self.get_flag(FLAG_Z),
    //         Target::Register8(Reg8::C) => self.get_flag(FLAG_C),
    //         Target::Register8(Reg8::NC) => !self.get_flag(FLAG_C),
    //         _ => true,
    //     }
    // }

    // Helper for the A-versions
    fn set_flags_rotate(&mut self, res: u8, carry: bool, is_a_version: bool) {
        self.set_flag(FLAG_Z, if is_a_version { false } else { res == 0 });
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry);
    }

    fn check_condition_operand(&self, target: Target) -> bool {
        // Note: You'll need to check if your build.rs maps
        // NZ/Z/NC/C to a specific Target variant.
        // If it currently maps them to Target::Register8(Reg8::C), etc:
        match target {
            Target::Register8(Reg8::C) => self.get_flag(FLAG_C),
            // ... handle others ...
            _ => true,
        }
    }
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
