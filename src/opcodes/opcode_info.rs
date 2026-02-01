use std::fmt;

use crate::cpu::AluOutput;

use super::*;

#[derive(Debug, Copy, Clone)]
pub struct OpcodeInfo {
    // Type is generated in build.rs
    pub mnemonic: Mnemonic,
    pub bytes: u8,
    pub bit_index: u8,
    pub cycles: &'static [u8],
    pub operands: &'static [(Target, bool)], // (Target, is_immediate)
    pub flags: FlagSpec,
}

impl OpcodeInfo {
    pub fn result(&self) -> InstructionResult {
        InstructionResult::from_instr(self)
    }
    /// For instructions that need to pass calculated flag proposals.
    pub fn result_with_flags(&self, z: bool, n: bool, h: bool, c: bool) -> InstructionResult {
        InstructionResult::with_flags(self, z, n, h, c)
    }

    pub fn result_with_alu(&self, alu: AluOutput) -> InstructionResult {
        InstructionResult::with_flags(self, alu.z, alu.n, alu.h, alu.c)
    }
    pub fn last_operand(&self) -> (Target, bool) {
        if self.operands.len() == 1 {
            self.operands[0]
        } else {
            self.operands[1]
        }
    }
}

impl fmt::Display for OpcodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write the mnemonic (e.g., "LD", "JP")
        write!(f, "{:<5}", format!("{:?}", self.mnemonic))?;

        // Format operands
        let operand_strings: Vec<String> = self
            .operands
            .iter()
            .map(|(target, _is_immediate)| target.as_string())
            .collect();

        if !operand_strings.is_empty() {
            // Max 40 characters.
            write!(f, " {: <29} {: >9}", operand_strings.join(", "), self.flags)?;
        }

        Ok(())
    }
}
