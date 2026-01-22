use std::fmt;

use super::*;

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

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())?;
        Ok(())
    }
}

impl Target {
    pub fn as_string(&self) -> String {
        match self {
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
}
