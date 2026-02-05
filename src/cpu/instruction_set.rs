// Coded with the help of Gemini.

use crate::cpu::Cpu;
use crate::cpu::alu::AluOutput;
use crate::mmu::Memory;
use crate::*;

impl InstructionSet for Cpu {
    fn nop(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        instruction.result()
    }
    fn add(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let dest_target = instruction.operands[0].0;
        let src_target = instruction.operands[1].0;

        match dest_target {
            // --- 16-BIT WORLD ---
            Target::Register16(Reg16::HL) | Target::StackPointer => {
                let val1 = self.read_target(dest_target, bus).as_u16();

                // Fix: Handle the signed I8 vs unsigned U16 operand
                let val2_raw = self.read_target(src_target, bus);
                let val2 = match val2_raw {
                    OperandValue::I8(v) => v as i16 as u16, // Sign-extend: e.g., -1 (0xFF) becomes 0xFFFF
                    _ => val2_raw.as_u16(),                 // Standard u16 for HL + BC/DE/HL/SP
                };

                let res = val1.wrapping_add(val2);
                self.write_target(dest_target, OperandValue::U16(res), bus);

                if dest_target == Target::StackPointer {
                    // Special Flag Case: ADD SP, e8
                    // Flags are based on the LOWER 8 bits of the addition!
                    let v1_low = (val1 & 0xFF) as u8;
                    let v2_low = (val2 & 0xFF) as u8;

                    instruction.result_with_flags(
                        false,                                  // Z always 0
                        false,                                  // N always 0
                        (v1_low & 0xF) + (v2_low & 0xF) > 0xF,  // H from bit 3
                        (v1_low as u16 + v2_low as u16) > 0xFF, // C from bit 7
                    )
                } else {
                    // Special Flag Case: ADD HL, r16
                    instruction.result_with_flags(
                        (self.f & FLAG_Z) != 0,                  // Z is unaffected (keep old value)
                        false,                                   // N always 0
                        (val1 & 0xFFF) + (val2 & 0xFFF) > 0xFFF, // H from bit 11
                        (val1 as u32 + val2 as u32) > 0xFFFF,    // C from bit 15
                    )
                }
            }

            // --- 8-BIT WORLD (Reg8 / Memory) ---
            _ => {
                // This handles ADD A, B | ADD A, C | ADD A, (HL) | ADD A, n8
                let val = self.read_target(src_target, bus).as_u8();
                let use_carry = instruction.mnemonic == Mnemonic::ADC;

                let res = self.alu_8bit_add(self.get_reg8(Reg8::A), val, use_carry);

                self.set_reg8(Reg8::A, res.value);
                instruction.result_with_flags(res.z, res.n, res.h, res.c)
            }
        }
    }

    fn jp(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let mut should_jump = true;
        let mut addr_index = 0;

        // 1. Check if the first operand is a condition
        if let (Target::Condition(cond), _) = instruction.operands[0] {
            should_jump = self.check_condition(Target::Condition(cond));
            addr_index = 1; // The address is the second operand
        }

        // 2. Always read the address (to advance PC), but only jump if condition is met
        let (addr_target, _) = instruction.operands[addr_index];
        let dest_addr = match addr_target {
            Target::Immediate16 | Target::AddrImmediate16 => {
                let val = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                val
            }
            Target::Register16(Reg16::HL) => self.get_reg16(Reg16::HL),
            _ => panic!("Unsupported JP address target: {:?}", addr_target),
        };

        if should_jump {
            self.pc = dest_addr;
            // Logic for "Jump Taken" cycles would go here if needed
        }

        InstructionResult::branching(&instruction, should_jump)
    }

    fn cp(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let val = self.get_src_val(&instruction, bus);

        let res = self.alu_8bit_sub(self.a, val, false);

        // CP ONLY updates Flags (A remains unchanged)
        instruction.result_with_flags(res.z, res.n, res.h, res.c)
    }

    fn jr(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (cond_target, offset_target) = if instruction.operands.len() == 2 {
            (Some(instruction.operands[0].0), instruction.operands[1].0)
        } else {
            (None, instruction.operands[0].0)
        };

        // ALWAYS read the offset to advance the PC past the instruction bytes
        let offset = self.read_target(offset_target, bus).as_i8();

        let cond_met = match cond_target {
            Some(t) => self.read_target(t, bus).as_bool(),
            None => true,
        };

        if cond_met {
            // Jump relative to the PC *after* it has been moved past the offset byte
            self.pc = self.pc.wrapping_add_signed(offset as i16);
        }

        InstructionResult::branching(&instruction, cond_met)
    }

    fn dec(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.last_operand();

        match target {
            // 8-bit Decrement (Affects Z, N, H)
            Target::Register8(reg) => {
                let val = self.get_reg8(reg);
                let alu = AluOutput::alu_8bit_dec(val);
                self.set_reg8(reg, alu.value);
                return instruction.result_with_alu(alu);
            }

            // 8-bit Memory Decrement (e.g., DEC (HL))
            Target::AddrRegister16(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read_byte(addr);
                let alu = AluOutput::alu_8bit_dec(val);
                bus.write_byte(addr, alu.value);
                return instruction.result_with_alu(alu);
            }

            // 16-bit Decrement (Affects NO flags)
            Target::Register16(reg) => {
                let val = self.get_reg16(reg);
                self.set_reg16(reg, val.wrapping_sub(1));
            }

            Target::StackPointer => {
                self.sp = self.sp.wrapping_sub(1);
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
        instruction.result_with_flags(res.z, res.n, res.h, res.c)
    }

    fn ld(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Check if we are dealing with the 3-operand special case (LD HL, SP + e8)
        if instruction.operands.len() == 3 {
            let (dest_target, _) = instruction.operands[0]; // HL
            let (sp_target, _) = instruction.operands[1]; // SP
            let (imm_target, _) = instruction.operands[2]; // e8

            // Read the two source values
            let sp = self.read_target(sp_target, bus).as_u16();
            let val_raw = self.read_target(imm_target, bus);

            // Convert signed immediate to 16-bit
            let r8 = match val_raw {
                OperandValue::I8(v) => v as i16 as u16,
                _ => val_raw.as_u16(),
            };

            let res = sp.wrapping_add(r8);

            // Write the result to HL
            self.write_target(dest_target, OperandValue::U16(res), bus);

            // Calculate those special low-byte flags
            let sp_low = (sp & 0xFF) as u8;
            let r8_low = (r8 & 0xFF) as u8;

            return instruction.result_with_flags(
                false,
                false,
                (sp_low & 0xF) + (r8_low & 0xF) > 0xF,
                (sp_low as u16 + r8_low as u16) > 0xFF,
            );
        }

        // 2. Standard 2-operand case
        let (dest, src) = (instruction.operands[0].0, instruction.operands[1].0);
        let val = self.read_target(src, bus);
        self.write_target(dest, val, bus);

        instruction.result()
    }

    fn xor(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let val = self.get_src_val(&instruction, bus);

        self.a ^= val;

        instruction.result_with_flags(self.a == 0, false, false, false)
    }
    fn or(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Get the destination target (usually A)
        let (dest_target, _) = instruction.operands[0];

        // 2. Get the source target (e.g., C, B, or an immediate u8)
        let (src_target, _) = instruction.operands[1];

        // 3. Read the values
        let current_val = self.read_target(dest_target, bus).as_u8();
        let operand_val = self.read_target(src_target, bus).as_u8();

        // 4. Perform the logic
        let res = current_val | operand_val;

        // 5. Write back to the destination defined in the spec
        self.write_target(dest_target, OperandValue::U8(res), bus);

        // 6. Return flag results
        instruction.result_with_flags(res == 0, false, false, false)
    }
    fn and(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let val = self.get_src_val(&instruction, bus);

        let res = self.get_reg8(Reg8::A) & val;
        self.set_reg8(Reg8::A, res);

        instruction.result_with_flags(res == 0, false, true, false)
    }
    fn di(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        self.ime = false;
        instruction.result()
    }
    fn ei(&mut self, instruction: OpcodeInfo, _bus: &mut impl Memory) -> InstructionResult {
        self.request_ime();
        instruction.result()
    }
    fn push(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (src, _) = instruction.operands[0];
        let val = self.get_reg16_from_target(src);

        // Stack grows downwards: Push High then Low
        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, ((val >> 8) & 0xFF) as u8);

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, (val & 0xFF) as u8);

        instruction.result()
    }
    fn pop(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (dest, _) = instruction.operands[0];

        // 1. Grab the 16-bit value from the stack (Little-Endian)
        let val = self.pop_u16(bus);

        // 2. Commit it to the registers (Big-Endian split: e.g. B=High, C=Low)
        self.set_reg16_from_target(dest, val);

        // 3. Special case for AF to update the supervisor's flag state
        if let Target::Register16(Reg16::AF) = dest {
            let f = (val & 0xFF) as u8;
            return instruction.result_with_flags(
                (f & FLAG_Z) != 0,
                (f & FLAG_N) != 0,
                (f & FLAG_H) != 0,
                (f & FLAG_C) != 0,
            );
        }

        instruction.result()
    }
    fn bit(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Get the bit index directly from your newly updated struct
        let bit = instruction.bit_index;

        // 2. Read the target value (could be a register or memory via (HL))
        let (src, _) = instruction.operands[1];
        let val = self.read_target(src, bus).as_u8();

        // 3. Check if the specific bit is set
        let is_set = (val & (1 << bit)) != 0;

        // 4. Update flags: Z is 1 if the bit was 0 (!is_set)
        instruction.result_with_flags(
            !is_set,               // Z
            false,                 // N
            true,                  // H (Always set for BIT)
            self.get_flag(FLAG_C), // C (Carry remains unchanged)
        )
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
        let (cond_target, _) = instruction.operands[0];

        // 1. Fetch the target address (3-byte instruction: Opcode + Low + High)
        let low = bus.read_byte(self.pc) as u16;
        let high = bus.read_byte(self.pc + 1) as u16;
        let target_addr = (high << 8) | low;

        let should_return = if self.check_condition(cond_target) {
            // Increment PC past the immediate address before pushing
            let return_addr = self.pc + 2;

            // Push return address to stack
            self.push_u16(bus, return_addr);

            // Perform the jump
            self.pc = target_addr;

            // Conditional CALL takes more cycles if it jumps (usually 24 vs 12)
            // Ensure your result reflects the "Taken" cycles if your system supports it.
            true
        } else {
            // If condition fails, we just skip the address bytes
            self.pc = self.pc.wrapping_add(2);
            false
        };

        InstructionResult::branching(&instruction, should_return)
    }
    fn ret(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Check if this is a conditional return
        // (In many specs, conditional RETs have a Condition as the first operand)
        let should_return = if let Some((Target::Condition(cond), _)) = instruction.operands.first()
        {
            self.check_condition(Target::Condition(*cond))
        } else {
            true // Unconditional RET (0xC9)
        };

        if should_return {
            // 2. Pop the address from the stack
            // 3. Jump to the return address
            self.pc = bus.read_u16(self.sp);
            self.sp = self.sp.wrapping_add(2);

            // Conditional RET usually takes 20 cycles if taken, 8 if not.
            // Unconditional RET is always 16.
        }

        // Note: If should_return is false, the PC remains at the
        // instruction after the RET (handled by your central step loop).
        // RET cycles are 20, 8. So we have to flip the conditional.
        InstructionResult::branching(&instruction, should_return)
    }
    fn inc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        match target {
            Target::Register8(reg) => {
                let val = self.get_reg8(reg);
                let res = val.wrapping_add(1);
                self.set_reg8(reg, res);
                return instruction.result_with_flags(res == 0, false, (val & 0x0F) == 0x0F, false);
            }
            Target::Register16(reg) => {
                let val = self.get_reg16(reg);
                self.set_reg16(reg, val.wrapping_add(1));
            }

            Target::StackPointer => {
                self.sp = self.sp.wrapping_add(1);
            }

            Target::AddrRegister16(reg) => {
                // 1. Get the address from the register (e.g., HL)
                let addr = self.get_reg16(reg);

                // 2. Read the value FROM memory at that address
                let val = bus.read_byte(addr);

                // 3. Increment the value
                let res = val.wrapping_add(1);

                // 4. Write the new value back to that same memory address
                bus.write_byte(addr, res);

                // 5. Update flags (Z, N=0, H, C is unaffected)
                return instruction.result_with_flags(
                    res == 0,             // Zero flag
                    false,                // Subtract flag (always reset for INC)
                    (val & 0x0F) == 0x0F, // Half-carry flag
                    false,                // Carry flag (NOT changed by 8-bit INC)
                );
            }

            _ => todo!("INC for {:?}", target),
        }
        instruction.result()
    }

    fn reti(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        // 1. Pop the PC from the stack (identical to RET)
        let low = bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = bus.read_byte(self.sp) as u16;
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
        instruction.result_with_flags(res.z, res.n, res.h, res.c)
    }
    fn set(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let bit = instruction.bit_index;
        let (target, _) = instruction.operands[1];

        let val = self.read_target(target, bus).as_u8();
        let res = val | (1 << bit); // Force the bit to 1

        self.write_target(target, OperandValue::U8(res), bus);
        instruction.result() // Return with no flag changes
    }

    fn res(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let bit = instruction.bit_index;
        let (target, _) = instruction.operands[1];

        let val = self.read_target(target, bus).as_u8();
        let res = val & !(1 << bit); // Force the bit to 0

        self.write_target(target, OperandValue::U8(res), bus);
        instruction.result() // Return with no flag changes
    }

    fn halt(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        if self.ime {
            self.halted = true;
        } else if bus.pending_interrupt() {
            // THE HALT BUG: IME is 0 and an interrupt is already pending.
            // The next instruction is "duplicated" or the PC fails to increment.
            self.halt_bug_triggered = true;
        } else {
            self.halted = true;
        }
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
                let offset = bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                bus.write_byte(0xFF00 + offset as u16, self.get_reg8(Reg8::A));
            }
            // LDH A, (n8) -> Load 0xFF00 + n8 into A
            (Target::Register8(Reg8::A), Target::AddrImmediate8) => {
                let offset = bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                let val = bus.read_byte(0xFF00 + offset as u16);
                self.set_reg8(Reg8::A, val);
            }
            // LDH A, (C) -> Load 0xFF00 + C into A
            (Target::Register8(Reg8::A), Target::AddrRegister8(Reg8::C)) => {
                let offset = self.get_reg8(Reg8::C);
                let val = bus.read_byte(0xFF00 + offset as u16);
                self.set_reg8(Reg8::A, val);
            }

            (Target::AddrRegister8(from), Target::Register8(to)) => {
                let offset = self.get_reg8(from);
                bus.write_byte(0xFF00 + offset as u16, self.get_reg8(to));
            }
            _ => todo!("LDH variant not handled"),
        }
        instruction.result()
    }
    fn rlca(&mut self, instruction: OpcodeInfo, _b: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);
        let carry = (a & 0x80) >> 7;
        let res = (a << 1) | carry;
        self.set_reg8(Reg8::A, res);

        // IMPORTANT: RLCA/RRCA/RLA/RRA set Z to 0, not (res == 0)
        instruction.result_with_flags(false, false, false, carry == 1)
    }

    fn rrca(&mut self, instruction: OpcodeInfo, _b: &mut impl Memory) -> InstructionResult {
        let a = self.get_reg8(Reg8::A);

        // 1. Get the bit that will be rotated out (bit 0)
        let bit0 = a & 0x01;

        // 2. Perform the rotation
        let res = (a >> 1) | (bit0 << 7);

        // 3. Update the Accumulator
        self.set_reg8(Reg8::A, res);

        instruction.result_with_flags(
            false,     // Z is forced to 0
            false,     // N is forced to 0
            false,     // H is forced to 0
            bit0 == 1, // Carry flag
        )
    }

    fn rlc(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let bit7 = (val & 0x80) >> 7;
        let res = (val << 1) | bit7;

        self.write_target(target, OperandValue::U8(res), bus);
        // self.set_flags_rotate(res, bit7 == 1, false); // false = CB-version
        instruction.result_with_flags(res == 0, false, false, bit7 == 1)
    }

    fn rl(&mut self, instruction: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {
        let (target, _) = instruction.operands[0];
        let val = self.read_target(target, bus).as_u8();
        let old_c = if self.get_flag(FLAG_C) { 1 } else { 0 };
        let new_c = (val & 0x80) >> 7;
        let res = (val << 1) | old_c;

        self.write_target(target, OperandValue::U8(res), bus);
        // self.set_flags_rotate(res, new_c == 1, false);
        instruction.result_with_flags(res == 0, false, false, new_c == 1)
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
        instruction.result_with_flags(res.z, res.n, res.h, res.c)
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
        bus.write_byte(self.sp, (pc >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, (pc & 0xFF) as u8);

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

        let res = val.rotate_left(4);
        self.write_target(target, OperandValue::U8(res), bus);

        // Pass all 4 flag proposals.
        // Spec [Z000] means:
        // - Z will be set to (res == 0)
        // - N, H, C will be forced to false (Reset) regardless of what you pass here.
        instruction.result_with_flags(res == 0, false, false, false)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::input::DummyInput;
    #[test]
    fn test_halt_bug_flag_activation() {
        let mut cpu = Cpu::new();
        let mut bus: Bus<DummyInput> = Bus::new(vec![0; 0x10000]);

        // 1. Define the HALT instruction metadata (adjust to your OpcodeInfo structure)
        let halt_info = OPCODES[0x76].unwrap();
        assert_eq!(halt_info.mnemonic, Mnemonic::HALT);

        // 2. Condition: IME is OFF
        cpu.ime = false;

        // 3. Condition: Interrupt is PENDING
        // Ensure both IE and IF have a matching bit set (e.g., V-Blank bit 0)
        bus.write_ie(0x01);
        bus.write_if(0x01);

        // 4. Call the halt function directly
        cpu.halt(halt_info, &mut bus);

        // 5. Verification
        assert!(
            cpu.halt_bug_triggered,
            "HALT bug should be triggered when IME=0 and Interrupt is pending"
        );
        assert!(
            !cpu.halted,
            "CPU should NOT enter halted state when the HALT bug occurs"
        );
    }
}
