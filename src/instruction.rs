use crate::cpu::Cpu;
use crate::memory_trait::Memory;
use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Target {
    StackPointer,
    Register8(Reg8),
    Register16(Reg16),
    Immediate8,
    Immediate16,
    AddrImmediate16,
    AddrImmediate8,        // for LDH (a8)
    AddrRegister8(Reg8),   // Add this for (C)
    AddrRegister16(Reg16), // This is for (HL), (BC), (DE)
    Bit(u8),
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperandValue {
    U8(u8),
    U16(u16),
}

impl OperandValue {
    pub fn as_u8(self) -> u8 {
        match self {
            OperandValue::U8(v) => v,
            _ => panic!("Expected u8 operand"),
        }
    }

    pub fn as_u16(self) -> u16 {
        match self {
            OperandValue::U16(v) => v,
            _ => panic!("Expected u16 operand"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Reg8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Reg16 {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
}

#[derive(Debug, Copy, Clone)]
pub struct OpcodeInfo {
    // Type is generated in build.rs
    pub mnemonic: Mnemonic,
    pub bytes: u8,
    pub cycles: &'static [u8],
    pub operands: &'static [(Target, bool)], // (Target, is_immediate)
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_target(self, false))?;
        Ok(())
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
            .map(|(target, is_immediate)| format_target(target, *is_immediate))
            .collect();

        if !operand_strings.is_empty() {
            write!(f, " {}", operand_strings.join(", "))?;
        }

        Ok(())
    }
}

/// Helper to turn Target into standard Assembly syntax
fn format_target(target: &Target, _is_immediate: bool) -> String {
    match target {
        Target::Register8(reg) => format!("{:?}", reg),
        Target::Register16(reg) => format!("{:?}", reg),
        Target::StackPointer => format!("sp"),
        Target::Immediate8 => "n8".to_string(),
        Target::Immediate16 => "n16".to_string(),
        Target::AddrImmediate16 => "(a16)".to_string(),
        Target::AddrImmediate8 => "(a8)".to_string(),
        Target::AddrRegister16(reg) => format!("({:?})", reg),
        Target::AddrRegister8(reg) => format!("({:?})", reg),
        Target::Bit(b) => format!("{}", b),
    }
}

// THE TRICK: Include the generated code right here.
// The generated code will "see" Target, Reg8, etc. because they are in scope.
include!(concat!(env!("OUT_DIR"), "/opcodes_generated.rs"));
