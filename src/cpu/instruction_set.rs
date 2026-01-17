// Coded with the help of Gemini.

use crate::cpu::*;
use crate::instruction::*;
use crate::memory_trait::Memory;
use crate::*;

impl InstructionSet for Cpu {
    fn nop(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        instruction.cycles[0]
    }
    fn add(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> u8 {
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

    fn jp(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> u8 {
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

    fn cp(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src_target, _) = instruction.operands[0]; // CP usually only lists the source
        let val = self.read_target(src_target, bus).as_u8();

        let res = self.alu_8bit_sub(self.a, val, false);

        // CP ONLY updates Flags (A remains unchanged)
        self.apply_alu_flags(res);

        instruction.cycles[0]
    }
    fn jr(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> u8 {
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

    fn dec(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instruction.operands[0];

        match target {
            // 8-bit Decrement (Affects Z, N, H)
            Target::Register8(reg) => {
                let val = self.get_reg8(reg);
                let (res, z, n, h) = self.calculate_dec_8bit(val);
                self.set_reg8(reg, res);
                self.set_flag(FLAG_Z, z);
                self.set_flag(FLAG_N, n);
                self.set_flag(FLAG_H, h);
            }

            // 8-bit Memory Decrement (e.g., DEC (HL))
            Target::AddrRegister16(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read(addr);
                let (res, z, n, h) = self.calculate_dec_8bit(val);
                bus.write(addr, res);
                self.set_flag(FLAG_Z, z);
                self.set_flag(FLAG_N, n);
                self.set_flag(FLAG_H, h);
            }

            // 16-bit Decrement (Affects NO flags)
            Target::Register16(reg) => {
                let val = self.get_reg16(reg);
                self.set_reg16(reg, val.wrapping_sub(1));
            }

            _ => panic!("DEC not implemented for target {:?}", target),
        }

        instruction.cycles[0]
    }

    fn sub(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src_target, _) = instruction.operands[1]; // A is usually operands[0]
        let val = self.read_target(src_target, bus).as_u8();

        let res = self.alu_8bit_sub(self.a, val, false);

        // SUB updates A and Flags
        self.a = res.value;
        self.apply_alu_flags(res);

        instruction.cycles[0]
    }

    fn ld(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (dest, src) = (instr.operands[0].0, instr.operands[1].0);

        // read_target should return OperandValue (U8 or U16)
        let val = self.read_target(src, bus);

        // write_target handles the routing
        self.write_target(dest, val, bus);

        instr.cycles[0]
    }

    fn xor(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src, _) = instr.operands[0];
        let val = self.read_target(src, bus).as_u8();

        self.a ^= val;

        // XOR Flags: Z if result 0, others always false
        self.set_flag(FLAG_Z, self.a == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, false);

        instr.cycles[0]
    }
    fn or(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src, _) = instr.operands[0];
        let val = self.read_target(src, bus).as_u8();

        let res = self.get_reg8(Reg8::A) | val;
        self.set_reg8(Reg8::A, res);

        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, false);

        instr.cycles[0]
    }
    fn and(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src, _) = instr.operands[0];
        let val = self.read_target(src, bus).as_u8();

        let res = self.get_reg8(Reg8::A) & val;
        self.set_reg8(Reg8::A, res);

        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true); // Unique to AND
        self.set_flag(FLAG_C, false);

        instr.cycles[0]
    }
    fn di(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        // IME = Interrupt Master Enable
        self.ime = false;
        instr.cycles[0]
    }
    fn ei(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        self.ime = true;
        instr.cycles[0]
    }
    fn push(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src, _) = instr.operands[0];
        let val = self.get_reg16_from_target(src);

        // Stack grows downwards: Push High then Low
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, ((val >> 8) & 0xFF) as u8);

        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (val & 0xFF) as u8);

        instr.cycles[0]
    }
    fn pop(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (dest, _) = instr.operands[0];

        let low = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        let high = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        let mut val = (high << 8) | low;

        // Quirk: Lower 4 bits of Flag register are always 0
        if let Target::Register16(Reg16::AF) = dest {
            val &= 0xFFF0;
        }

        self.set_reg16_from_target(dest, val);
        instr.cycles[0]
    }
    fn bit(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        // BIT b, r8 (e.g., BIT 7, H)
        let bit_index = match instr.operands[0].0 {
            Target::Immediate8 => bus.read(self.pc - 1), // Simplification depending on your decoder
            _ => 0, // In many JSONs, the bit is embedded in the instruction metadata
        };
        let (src, _) = instr.operands[1];
        let val = self.read_target(src, bus).as_u8();

        let is_set = (val & (1 << bit_index)) != 0;

        self.set_flag(FLAG_Z, !is_set);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, true); // BIT always sets H to true
        // C flag is left unchanged

        instr.cycles[0]
    }
    fn cpl(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        let a = self.get_reg8(Reg8::A);
        self.set_reg8(Reg8::A, !a);

        // Flags: Z is unaffected, N and H become true
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, true);

        instr.cycles[0]
    }
    fn scf(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, true);

        instr.cycles[0]
    }
    fn call(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        // 1. Read the address we are jumping to
        let low = bus.read(self.pc) as u16;
        let high = bus.read(self.pc + 1) as u16;
        let target_addr = (high << 8) | low;

        // 2. Push the address of the NEXT instruction onto the stack
        let return_addr = self.pc + 2;
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (return_addr >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (return_addr & 0xFF) as u8);

        // 3. Jump
        self.pc = target_addr;

        instr.cycles[0]
    }
    fn ret(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let low = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        self.pc = (high << 8) | low;

        instr.cycles[0]
    }
    fn inc(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        match target {
            Target::Register8(reg) => {
                let val = self.get_reg8(reg);
                let res = val.wrapping_add(1);
                self.set_reg8(reg, res);
                self.set_flag(FLAG_Z, res == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (val & 0x0F) == 0x0F);
            }
            Target::Register16(reg) => {
                let val = self.get_reg16(reg);
                self.set_reg16(reg, val.wrapping_add(1));
            }
            _ => todo!("INC for {:?}", target),
        }
        instr.cycles[0]
    }

    fn reti(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        // 1. Pop the PC from the stack (identical to RET)
        let low = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        self.pc = (high << 8) | low;

        // 2. Immediately enable interrupts
        self.ime = true;

        instr.cycles[0]
    }
    fn adc(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src, _) = instr.operands[1];
        let val = self.read_target(src, bus).as_u8();

        // Reuse your ALU helper
        let res = self.alu_8bit_add(self.get_reg8(Reg8::A), val, true);

        self.set_reg8(Reg8::A, res.value);
        self.apply_alu_flags(res);

        instr.cycles[0]
    }
    fn set(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        // Operand 0 is the bit index (0-7), Operand 1 is the target
        let bit_index = match instr.operands[0].0 {
            Target::Immediate8 => bus.read(self.pc - 1), // Check your decoder's specific implementation
            _ => 0,
        };
        let (target, _) = instr.operands[1];
        let val = self.read_target(target, bus).as_u8();

        let res = val | (1 << bit_index);
        self.write_target(target, OperandValue::U8(res), bus);

        instr.cycles[0]
    }
    fn res(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let bit_index = match instr.operands[0].0 {
            Target::Immediate8 => bus.read(self.pc - 1),
            _ => 0,
        };
        let (target, _) = instr.operands[1];
        let val = self.read_target(target, bus).as_u8();

        let res = val & !(1 << bit_index);
        self.write_target(target, OperandValue::U8(res), bus);

        instr.cycles[0]
    }
    fn halt(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        // If IME is enabled, the CPU stops until an interrupt occurs.
        // If IME is disabled, there is a famous "Halt Bug" (skipping the next byte).
        // For now, let's keep it simple:
        self.halted = true;

        instr.cycles[0]
    }
    fn daa(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        let mut a = self.get_reg8(Reg8::A);
        let mut adjust = 0;
        let mut carry = false;

        if !self.get_flag(FLAG_N) {
            // After an ADD
            if self.get_flag(FLAG_C) || a > 0x99 {
                adjust |= 0x60;
                carry = true;
            }
            if self.get_flag(FLAG_H) || (a & 0x0F) > 0x09 {
                adjust |= 0x06;
            }
        } else {
            // After a SUB
            if self.get_flag(FLAG_C) {
                adjust |= 0x60;
                carry = true;
            }
            if self.get_flag(FLAG_H) {
                adjust |= 0x06;
            }
            // Subtraction adjust is effectively negative
            a = a.wrapping_sub(adjust);
        }

        if !self.get_flag(FLAG_N) {
            a = a.wrapping_add(adjust);
        }

        self.set_reg8(Reg8::A, a);
        self.set_flag(FLAG_Z, a == 0);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry);

        instr.cycles[0]
    }
    fn sla(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let carry = (val & 0x80) != 0;
        let res = val << 1;

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry);

        instr.cycles[0]
    }
    fn srl(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let carry = (val & 0x01) != 0;
        let res = val >> 1;

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry);

        instr.cycles[0]
    }
    fn sra(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let carry = (val & 0x01) != 0;
        // Bit 7 stays the same, other bits shift right
        let res = (val >> 1) | (val & 0x80);

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, carry);

        instr.cycles[0]
    }
    fn ccf(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        let c = self.get_flag(FLAG_C);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, !c);

        instr.cycles[0]
    }
    fn ldh(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (dest, src) = (instr.operands[0].0, instr.operands[1].0);

        match (dest, src) {
            // LDH (n8), A -> Store A into 0xFF00 + n8
            (Target::AddrImmediate8, Target::Register8(Reg8::A)) => {
                let offset = bus.read(self.pc);
                self.pc = self.pc.wrapping_add(1);
                bus.write(0xFF00 + offset as u16, self.get_reg8(Reg8::A));
            }
            // LDH A, (n8) -> Load 0xFF00 + n8 into A
            (Target::Register8(Reg8::A), Target::AddrImmediate8) => {
                let offset = bus.read(self.pc);
                self.pc = self.pc.wrapping_add(1);
                let val = bus.read(0xFF00 + offset as u16);
                self.set_reg8(Reg8::A, val);
            }
            // LDH A, (C) -> Load 0xFF00 + C into A
            (Target::Register8(Reg8::A), Target::AddrRegister8(Reg8::C)) => {
                let offset = self.get_reg8(Reg8::C);
                let val = bus.read(0xFF00 + offset as u16);
                self.set_reg8(Reg8::A, val);
            }
            _ => todo!("LDH variant not handled"),
        }
        instr.cycles[0]
    }
    fn rlca(&mut self, instr: OpcodeInfo, _b: &mut impl Memory) -> u8 {
        let a = self.get_reg8(Reg8::A);
        let bit7 = (a & 0x80) >> 7;
        let res = (a << 1) | bit7;
        self.set_reg8(Reg8::A, res);
        self.set_flags_rotate(res, bit7 == 1, true); // true = A-version
        instr.cycles[0]
    }

    fn rrca(&mut self, instr: OpcodeInfo, _b: &mut impl Memory) -> u8 {
        let a = self.get_reg8(Reg8::A);
        let bit0 = a & 0x01;
        let res = (a >> 1) | (bit0 << 7);
        self.set_reg8(Reg8::A, res);
        self.set_flags_rotate(res, bit0 == 1, true);
        instr.cycles[0]
    }
    fn rlc(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let bit7 = (val & 0x80) >> 7;
        let res = (val << 1) | bit7;

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flags_rotate(res, bit7 == 1, false); // false = CB-version
        instr.cycles[0]
    }

    fn rl(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = (val & 0x80) >> 7;
        let res = (val << 1) | old_c;

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flags_rotate(res, new_c == 1, false);
        instr.cycles[0]
    }
    fn stop(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        self.halted = true; // For now, treat like HALT
        // Real hardware would also stop the oscillator
        instr.cycles[0]
    }
    fn sbc(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (src, _) = instr.operands[1]; // Typically SBC A, r8
        let val = self.read_target(src, bus).as_u8();

        // Reuse the unified ALU helper we built earlier
        let res = self.alu_8bit_sub(self.get_reg8(Reg8::A), val, true);

        self.set_reg8(Reg8::A, res.value);
        self.apply_alu_flags(res);

        instr.cycles[0]
    }
    fn rla(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        let a = self.get_reg8(Reg8::A);
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = (a & 0x80) >> 7;

        let res = (a << 1) | old_c;
        self.set_reg8(Reg8::A, res);

        self.set_flag(FLAG_Z, false); // Always false for RLA
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, new_c == 1);

        instr.cycles[0]
    }

    fn rra(&mut self, instr: OpcodeInfo, _bus: &mut impl Memory) -> u8 {
        let a = self.get_reg8(Reg8::A);
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = a & 0x01;

        let res = (a >> 1) | (old_c << 7);
        self.set_reg8(Reg8::A, res);

        self.set_flag(FLAG_Z, false); // Always false for RRA
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, new_c == 1);

        instr.cycles[0]
    }
    fn rr(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = val & 0x01;

        let res = (val >> 1) | (old_c << 7);
        self.write_target(target, OperandValue::U8(res), bus);

        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, new_c == 1);

        instr.cycles[0]
    }

    fn rrc(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let bit0 = val & 0x01;

        let res = (val >> 1) | (bit0 << 7);
        self.write_target(target, OperandValue::U8(res), bus);

        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, bit0 == 1);

        instr.cycles[0]
    }
    fn rst(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        // 1. Push current PC to stack
        let pc = self.pc;
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (pc >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (pc & 0xFF) as u8);

        // 2. The target address is usually part of the mnemonic (e.g., RST 00h)
        // or passed as an immediate by your decoder.
        let (target, _) = instr.operands[0];
        let vector = self.read_target(target, bus).as_u16();
        self.pc = vector;

        instr.cycles[0]
    }
    fn swap(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {
        let (target, _) = instr.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let res = (val >> 4) | (val << 4);
        self.write_target(target, OperandValue::U8(res), bus);

        self.set_flag(FLAG_Z, res == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, false);

        instr.cycles[0]
    }
}
