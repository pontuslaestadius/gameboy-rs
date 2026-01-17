pub mod immediate;
pub mod opcode;
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
}
struct AluResult {
    value: u8,
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}
impl Cpu {
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
            self.execute(code, bus);
        }
    }

    fn add(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        // For ADD A, r8 or ADD A, n8
        // Operands[0] is A, Operands[1] is the source
        let src_index = if instruction.operands.len() > 1 { 1 } else { 0 };
        let (src_target, _) = instruction.operands[src_index];

        let val = self.read_target(src_target, bus).as_u8();

        // Check if mnemonic is ADC (Add with Carry) or regular ADD
        let use_carry = instruction.mnemonic == Mnemonic::ADC;

        let res = self.alu_8bit_add(self.a, val, use_carry);

        // Update A and Flags
        self.a = res.value;
        self.apply_alu_flags(res);

        instruction.cycles[0]
    }

    fn jp(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        let (target, _) = instruction.operands[0];
        let dest_addr = match target {
            // If it's n16 or a16, we just want the 16-bit immediate value from the bus
            Target::Immediate16 | Target::AddrImmediate16 => {
                let val = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                val
            }
            Target::Register16(Reg16::HL) => self.get_reg16(Reg16::HL),
            _ => panic!("Unsupported JP target: {:?}", target),
        };

        self.pc = dest_addr;
        instruction.cycles[0]
    }

    fn cp(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        let (src_target, _) = instruction.operands[0]; // CP usually only lists the source
        let val = self.read_target(src_target, bus).as_u8();

        let res = self.alu_8bit_sub(self.a, val, false);

        // CP ONLY updates Flags (A remains unchanged)
        self.apply_alu_flags(res);

        instruction.cycles[0]
    }
    fn jr(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        // If only 1 operand, it's unconditional.
        // If 2 operands, [0] is condition, [1] is offset.
        let (target, _) = if instruction.operands.len() == 2 {
            // For now, let's assume we skip the condition if we don't have the enum
            instruction.operands[1]
        } else {
            instruction.operands[0]
        };

        let offset = self.read_target(target, bus).as_u8() as i8;

        // Always branch for now to keep the trace moving
        self.pc = self.pc.wrapping_add_signed(offset as i16);

        instruction.cycles[0]
    }

    fn sub(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        let (src_target, _) = instruction.operands[1]; // A is usually operands[0]
        let val = self.read_target(src_target, bus).as_u8();

        let res = self.alu_8bit_sub(self.a, val, false);

        // SUB updates A and Flags
        self.a = res.value;
        self.apply_alu_flags(res);

        instruction.cycles[0]
    }

    fn ld(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
        let (dest_target, _) = instruction.operands[0];
        let (src_target, _) = instruction.operands[1];

        // 1. Read the value from the source (e.g., could be Register A or a memory address)
        let value = self.read_target(src_target, bus);

        // 2. Write that value to the destination (e.g., 0xFF00 + a8)
        self.write_target(dest_target, value, bus);

        instruction.cycles[0]
    }

    fn execute(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) {
        info!("CPU: {}", self);
        info!("{:#X}. {}", self.pc, instruction);
        let cycles = match instruction.mnemonic {
            Mnemonic::JP => self.jp(instruction, bus),
            Mnemonic::CP => self.cp(instruction, bus),
            Mnemonic::LD | Mnemonic::LDH => self.ld(instruction, bus),
            Mnemonic::SUB => self.sub(instruction, bus),
            Mnemonic::JR => self.jr(instruction, bus),
            Mnemonic::ADD => {
                let (dest, _) = instruction.operands[0];
                match dest {
                    Target::Register16(Reg16::HL) => panic!("Not supported."),
                    _ => self.add(instruction, bus),
                }
            }

            Mnemonic::HALT => {
                self.halted = true;
                instruction.cycles[0]
            }
            Mnemonic::STOP => {
                // STOP is technically a 2-byte instruction (0x10 00),
                // but many emulators treat it as a special halt.
                self.halted = true;
                instruction.cycles[0]
            }
            Mnemonic::SCF => {
                // Set Carry Flag
                self.set_flag(FLAG_C, true);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                instruction.cycles[0]
            }
            Mnemonic::CCF => {
                // Complement Carry Flag (Flip it)
                let c = self.get_flag(FLAG_C);
                self.set_flag(FLAG_C, !c);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                instruction.cycles[0]
            }
            Mnemonic::CPL => {
                // Complement Accumulator (A = NOT A)
                self.a = !self.a;
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, true);
                instruction.cycles[0]
            }
            Mnemonic::DI => {
                self.ime = false;
                instruction.cycles[0]
            }
            Mnemonic::EI => {
                self.ime = true;
                instruction.cycles[0]
            }
            Mnemonic::NOP => instruction.cycles[0],
            _ => {
                panic!(
                    "No handler exists for {:?} | pc: {}",
                    instruction.mnemonic, self.pc
                );
            }
        };

        // If there are two, use index 0 for branched, index 1 for not branched.
        std::thread::sleep(T_CYCLE * cycles as u32);
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
            (Target::AddrRegister16(Reg16::HL), OperandValue::U8(v)) => {
                let addr = self.get_reg16(Reg16::HL);
                mmu.write(addr, v);
            }
            // a16 is a common write target (e.g., LD (a16), SP)
            (Target::AddrImmediate16, _) => {
                // You'd need to fetch the address from PC first
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
            _ => panic!("Invalid write target or value mismatch"),
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
