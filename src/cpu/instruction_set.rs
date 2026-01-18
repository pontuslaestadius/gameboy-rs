// Coded with the help of Gemini.

use crate::cpu::*;
use crate::instruction::*;
use crate::memory_trait::Memory;
use crate::*;

impl InstructionSet for Cpu {
    fn nop(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        instruction.result()
    }
    fn add(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let dest_target = instruction.operands[0].0;
        let src_target = instruction.operands[1].0;

        // Check if we are doing 16-bit addition (Target is HL or SP)
        match dest_target {
            Target::Register16(Reg16::HL) | Target::StackPointer => {
                let val1 = self.read_target(dest_target, bus).as_u16();
                let val2 = self.read_target(src_target, bus).as_u16();

                // 16-bit ADD logic (HL = HL + r16)
                let res = val1.wrapping_add(val2);

                // Flags for ADD HL, rr:
                // Z: Not affected!
                // N: Reset (0)
                // H: Set if carry from bit 11
                // C: Set if carry from bit 15
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (val1 & 0xFFF) + (val2 & 0xFFF) > 0xFFF);
                self.set_flag(FLAG_C, (val1 as u32 + val2 as u32) > 0xFFFF);

                self.write_target(dest_target, OperandValue::U16(res), bus);
            }
            _ => {
                // 8-bit logic for ADD A, r8
                let val = self.read_target(src_target, bus).as_u8();
                let use_carry = instruction.mnemonic == Mnemonic::ADC;
                let res = self.alu_8bit_add(self.a, val, use_carry);
                self.a = res.value;
                self.apply_alu_flags(res);
            }
        }

        instruction.result()
    }

    fn jp(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
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
        instruction.result()
    }

    fn cp(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src_target, _) = instruction.operands[0]; // CP usually only lists the source
        let val = self.read_target(src_target, bus).as_u8();

        let res = self.alu_8bit_sub(self.a, val, false);

        // CP ONLY updates Flags (A remains unchanged)
        self.apply_alu_flags(res);

        instruction.result()
    }
    fn jr(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // If there's only one operand (JR e8), it's an unconditional jump.
        // If there are two (JR NZ, e8), the first is the condition.
        let (cond_met, offset) = if instruction.operands.len() == 2 {
            (
                self.read_target(instruction.operands[0].0, bus).as_bool(),
                self.read_target(instruction.operands[1].0, bus).as_i8(),
            )
        } else {
            (
                true,
                self.read_target(instruction.operands[0].0, bus).as_i8(),
            )
        };

        if cond_met {
            // Use wrapping_add_signed to safely handle the i8 offset
            self.pc = self.pc.wrapping_add_signed(offset as i16);
        }

        InstructionResult::branching(&instruction, cond_met)
    }

    fn dec(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];

        match target {
            // 8-bit Decrement (Affects Z, N, H)
            Target::Register8(reg) => {
                let val = self.get_reg8(reg);
                let (res, z, n, h) = self.calculate_dec_8bit(val);
                self.set_reg8(reg, res);
                self.set_z(z);
                self.set_n(n);
                self.set_h(h);
            }

            // 8-bit Memory Decrement (e.g., DEC (HL))
            Target::AddrRegister16(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read(addr);
                let (res, z, n, h) = self.calculate_dec_8bit(val);
                bus.write(addr, res);
                self.set_z(z);
                self.set_n(n);
                self.set_h(h);
            }

            // 16-bit Decrement (Affects NO flags)
            Target::Register16(reg) => {
                let val = self.get_reg16(reg);
                self.set_reg16(reg, val.wrapping_sub(1));
            }

            _ => panic!("DEC not implemented for target {:?}", target),
        }

        instruction.result()
    }

    fn sub(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src_target, _) = instruction.operands[1]; // A is usually operands[0]
        let val = self.read_target(src_target, bus).as_u8();

        let res = self.alu_8bit_sub(self.a, val, false);

        // SUB updates A and Flags
        self.a = res.value;
        self.apply_alu_flags(res);

        instruction.result()
    }

    fn ld(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (dest, src) = (instruction.operands[0].0, instruction.operands[1].0);

        // read_target should return OperandValue (U8 or U16)
        let val = self.read_target(src, bus);

        // write_target handles the routing
        self.write_target(dest, val, bus);

        instruction.result()
    }

    fn xor(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[0];
        let val = self.read_target(src, bus).as_u8();

        self.a ^= val;

        // XOR Flags: Z if result 0, others always false
        self.set_z(self.a == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);

        instruction.result()
    }
    fn or(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[0];
        let val = self.read_target(src, bus).as_u8();

        let res = self.get_reg8(Reg8::A) | val;
        self.set_reg8(Reg8::A, res);

        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(false);
        self.set_c(false);

        instruction.result()
    }
    fn and(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[0];
        let val = self.read_target(src, bus).as_u8();

        let res = self.get_reg8(Reg8::A) & val;
        self.set_reg8(Reg8::A, res);

        self.set_z(res == 0);
        self.set_n(false);
        self.set_h(true); // Unique to AND
        self.set_c(false);

        instruction.result()
    }
    fn di(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        self.ime = false;
        instruction.result()
    }
    fn ei(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        self.ime = true;
        instruction.result()
    }
    fn push(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[0];
        let val = self.get_reg16_from_target(src);

        // Stack grows downwards: Push High then Low
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, ((val >> 8) & 0xFF) as u8);

        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (val & 0xFF) as u8);

        instruction.result()
    }
    fn pop(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (dest, _) = instruction.operands[0];

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
        instruction.result()
    }
    fn bit(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // BIT b, r8 (e.g., BIT 7, H)
        let bit_index = match instruction.operands[0].0 {
            Target::Immediate8 => bus.read(self.pc - 1), // Simplification depending on your decoder
            _ => 0, // In many JSONs, the bit is embedded in the instruction metadata
        };
        let (src, _) = instruction.operands[1];
        let val = self.read_target(src, bus).as_u8();

        let is_set = (val & (1 << bit_index)) != 0;

        self.set_z(!is_set);
        self.set_n(false);
        self.set_h(true); // BIT always sets H to true
        // C flag is left unchanged

        instruction.result()
    }
    fn cpl(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);
        self.set_reg8(Reg8::A, !a);

        // Flags: Z is unaffected, N and H become true
        self.set_n(true);
        self.set_h(true);

        instruction.result()
    }
    fn scf(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        // FlagAction::Set for C and FlagAction::Reset for N/H
        // are handled by your step function.
        instruction.result()
    }
    fn call(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
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

        instruction.result()
    }
    fn ret(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let low = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        self.pc = (high << 8) | low;

        instruction.result()
    }
    fn inc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        match target {
            Target::Register8(reg) => {
                let val = self.get_reg8(reg);
                let res = val.wrapping_add(1);
                self.set_reg8(reg, res);
                self.set_z(res == 0);
                self.set_n(false);
                self.set_h((val & 0x0F) == 0x0F);
            }
            Target::Register16(reg) => {
                let val = self.get_reg16(reg);
                self.set_reg16(reg, val.wrapping_add(1));
            }
            _ => todo!("INC for {:?}", target),
        }
        instruction.result()
    }

    fn reti(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Pop the PC from the stack (identical to RET)
        let low = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        self.pc = (high << 8) | low;

        // 2. Immediately enable interrupts
        self.ime = true;

        instruction.result()
    }
    fn adc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[1];
        let val = self.read_target(src, bus).as_u8();

        // Reuse your ALU helper
        let res = self.alu_8bit_add(self.get_reg8(Reg8::A), val, true);

        self.set_reg8(Reg8::A, res.value);
        self.apply_alu_flags(res);

        instruction.result()
    }
    fn set(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // Operand 0 is the bit index (0-7), Operand 1 is the target
        let bit_index = match instruction.operands[0].0 {
            Target::Immediate8 => bus.read(self.pc - 1), // Check your decoder's specific implementation
            _ => 0,
        };
        let (target, _) = instruction.operands[1];
        let val = self.read_target(target, bus).as_u8();

        let res = val | (1 << bit_index);
        self.write_target(target, OperandValue::U8(res), bus);

        instruction.result()
    }
    fn res(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let bit_index = match instruction.operands[0].0 {
            Target::Immediate8 => bus.read(self.pc - 1),
            _ => 0,
        };
        let (target, _) = instruction.operands[1];
        let val = self.read_target(target, bus).as_u8();

        let res = val & !(1 << bit_index);
        self.write_target(target, OperandValue::U8(res), bus);

        instruction.result()
    }
    fn halt(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        // If IME is enabled, the CPU stops until an interrupt occurs.
        // If IME is disabled, there is a famous "Halt Bug" (skipping the next byte).
        // For now, let's keep it simple:
        self.halted = true;

        instruction.result()
    }
    fn daa(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
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
        instruction.result_with_flags(a == 0, false, false, carry)
    }
    fn sla(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let carry = (val & 0x80) != 0;
        let res = val << 1;

        self.write_target(target, OperandValue::U8(res), bus);
        instruction.result_with_flags(res == 0, false, false, carry)
    }
    fn srl(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let carry = (val & 0x01) != 0;
        let res = val >> 1;

        self.write_target(target, OperandValue::U8(res), bus);
        instruction.result_with_flags(res == 0, false, false, carry)
    }
    fn sra(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let carry = (val & 0x01) != 0;
        // Bit 7 stays the same, other bits shift right
        let res = (val >> 1) | (val & 0x80);

        self.write_target(target, OperandValue::U8(res), bus);
        instruction.result_with_flags(res == 0, false, false, carry)
    }

    fn ccf(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        instruction.result()
    }
    fn ldh(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (dest, src) = (instruction.operands[0].0, instruction.operands[1].0);

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
        instruction.result()
    }
    fn rlca(&mut self, instruction: OpcodeInfo, _b: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);
        let bit7 = (a & 0x80) >> 7;
        let res = (a << 1) | bit7;
        self.set_reg8(Reg8::A, res);
        self.set_flags_rotate(res, bit7 == 1, true); // true = A-version
        instruction.result()
    }

    fn rrca(&mut self, instruction: OpcodeInfo, _b: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);
        let bit0 = a & 0x01;
        let res = (a >> 1) | (bit0 << 7);
        self.set_reg8(Reg8::A, res);
        self.set_flags_rotate(res, bit0 == 1, true);
        instruction.result()
    }
    fn rlc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let bit7 = (val & 0x80) >> 7;
        let res = (val << 1) | bit7;

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flags_rotate(res, bit7 == 1, false); // false = CB-version
        instruction.result()
    }

    fn rl(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = (val & 0x80) >> 7;
        let res = (val << 1) | old_c;

        self.write_target(target, OperandValue::U8(res), bus);
        self.set_flags_rotate(res, new_c == 1, false);
        instruction.result()
    }
    fn stop(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        self.halted = true; // For now, treat like HALT
        // Real hardware would also stop the oscillator
        instruction.result()
    }
    fn sbc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[1]; // Typically SBC A, r8
        let val = self.read_target(src, bus).as_u8();

        // Reuse the unified ALU helper we built earlier
        let res = self.alu_8bit_sub(self.get_reg8(Reg8::A), val, true);

        self.set_reg8(Reg8::A, res.value);
        self.apply_alu_flags(res);

        instruction.result()
    }
    fn rla(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = (a & 0x80) >> 7;

        let res = (a << 1) | old_c;
        self.set_reg8(Reg8::A, res);

        instruction.result_with_flags(false, false, false, new_c == 1)
    }

    fn rra(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = a & 0x01;

        let res = (a >> 1) | (old_c << 7);
        self.set_reg8(Reg8::A, res);

        instruction.result_with_flags(false, false, false, new_c == 1)
    }
    fn rr(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = val & 0x01;

        let res = (val >> 1) | (old_c << 7);
        self.write_target(target, OperandValue::U8(res), bus);

        instruction.result_with_flags(res == 0, false, false, new_c == 1)
    }

    fn rrc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let bit0 = val & 0x01;

        let res = (val >> 1) | (bit0 << 7);
        self.write_target(target, OperandValue::U8(res), bus);

        instruction.result_with_flags(res == 0, false, false, bit0 == 1)
    }
    fn rst(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Push current PC to stack
        let pc = self.pc;
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (pc >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (pc & 0xFF) as u8);

        // 2. The target address is usually part of the mnemonic (e.g., RST 00h)
        // or passed as an immediate by your decoder.
        let (target, _) = instruction.operands[0];
        let vector = self.read_target(target, bus).as_u16();
        self.pc = vector;

        instruction.result()
    }
    fn swap(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();

        let res = (val >> 4) | (val << 4);
        self.write_target(target, OperandValue::U8(res), bus);

        // Pass all 4 flag proposals.
        // Spec [Z000] means:
        // - Z will be set to (res == 0)
        // - N, H, C will be forced to false (Reset) regardless of what you pass here.
        instruction.result_with_flags(res == 0, false, false, false)
    }
}
