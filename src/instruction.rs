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
    AddrImmediate8,                 // for LDH (a8)
    AddrRegister8(Reg8),            // Add this for (C)
    AddrRegister16(Reg16),          // This is for (HL), (BC), (DE)
    AddrRegister16Decrement(Reg16), // (HL-)
    AddrRegister16Increment(Reg16), // (HL+)
    Bit(u8),
    Condition(Condition), // New: For conditional jumps
    Relative8,            // New: Signed i8 for JR instructions
    Vector(u8),           // New: Fixed addresses for RST
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Condition {
    NotZero,
    Zero,
    NotCarry,
    Carry,
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperandValue {
    U8(u8),
    U16(u16),
    I8(i8),     // Needed for Relative8 (JR offsets)
    Bool(bool), // Needed for Conditions (NZ, Z, etc.)
}

impl OperandValue {
    pub fn as_u8(self) -> u8 {
        match self {
            OperandValue::U8(v) => v,
            _ => panic!("Expected U8, got {:?}", self),
        }
    }

    pub fn as_u16(self) -> u16 {
        match self {
            OperandValue::U16(v) => v,
            OperandValue::U8(v) => v as u16, // Safe promotion
            _ => panic!("Expected U16, got {:?}", self),
        }
    }

    pub fn as_i8(self) -> i8 {
        match self {
            OperandValue::I8(v) => v,
            _ => panic!("Expected I8, got {:?}", self),
        }
    }

    pub fn as_bool(self) -> bool {
        match self {
            OperandValue::Bool(v) => v,
            _ => panic!("Expected Bool, got {:?}", self),
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

fn format_target(target: &Target, _is_immediate: bool) -> String {
    match target {
        Target::Register8(reg) => format!("{:?}", reg),
        Target::Register16(reg) => format!("{:?}", reg),
        Target::StackPointer => "sp".to_string(),
        Target::Immediate8 => "n8".to_string(),
        Target::Immediate16 => "n16".to_string(),
        Target::AddrImmediate16 => "(a16)".to_string(),
        Target::AddrImmediate8 => "(a8)".to_string(),
        Target::AddrRegister16(reg) => format!("({:?})", reg),
        Target::AddrRegister8(reg) => format!("({:?})", reg),
        Target::Bit(b) => format!("{}", b),
        Target::AddrRegister16Decrement(reg) => format!("({:?}-)", reg),
        Target::AddrRegister16Increment(reg) => format!("({:?}+)", reg),

        // --- New Target Mappings ---

        // Maps Condition to NZ, Z, NC, C
        Target::Condition(cond) => match cond {
            Condition::NotZero => "NZ".to_string(),
            Condition::Zero => "Z".to_string(),
            Condition::NotCarry => "NC".to_string(),
            Condition::Carry => "C".to_string(),
        },

        // 'e8' represents a signed 8-bit relative offset
        Target::Relative8 => "e8".to_string(),

        // Vectors are usually printed as hex addresses (e.g., 00h or $00)
        Target::Vector(v) => format!("${:02X}", v),
    }
}
// THE TRICK: Include the generated code right here.
// The generated code will "see" Target, Reg8, etc. because they are in scope.
include!(concat!(env!("OUT_DIR"), "/opcodes_generated.rs"));
