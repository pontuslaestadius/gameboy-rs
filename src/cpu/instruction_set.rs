use crate::cpu::Cpu;
use crate::instruction::*;
use crate::*;

impl InstructionSet for Cpu {
    fn nop(&mut self, instruction: OpcodeInfo, _bus: &mut impl memory_trait::Memory) -> u8 {
        instruction.cycles[0]
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

    fn dec(&mut self, instruction: OpcodeInfo, bus: &mut impl memory_trait::Memory) -> u8 {
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
}
