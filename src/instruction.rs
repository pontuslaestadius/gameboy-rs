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
pub struct InstructionResult {
    pub cycles: u8,
    pub z: bool, // Proposed Zero
    pub h: bool, // Proposed Half-Carry
    pub c: bool, // Proposed Carry
    pub n: bool,
}

impl InstructionResult {
    /// Use this for instructions with fixed timing (LD, NOP, ALU, etc.)
    /// It automatically pulls the first cycle value from the opcode metadata.
    pub fn from_instr(instr: &OpcodeInfo) -> Self {
        Self {
            cycles: instr.cycles[0],
            z: false,
            h: false,
            c: false,
            n: false,
        }
    }

    /// Use this for arithmetic where flags are calculated.
    /// It pulls the cycles and lets you propose flag states.
    pub fn with_flags(instr: &OpcodeInfo, z: bool, n: bool, h: bool, c: bool) -> Self {
        Self {
            cycles: instr.cycles[0],
            z,
            h,
            c,
            n,
        }
    }

    /// Use this for conditional branches (JR, JP, CALL, RET).
    /// Pass 'condition_met' to automatically select the correct timing from the JSON.
    pub fn branching(instr: &OpcodeInfo, condition_met: bool) -> Self {
        Self {
            cycles: if condition_met {
                instr.cycles[0]
            } else {
                instr.cycles[1]
            },
            z: false,
            h: false,
            c: false,
            n: false,
        }
    }
    /// For instructions like LD, JP, NOP that do not change flags.
    /// The flags will be ignored by the FlagSpec filter (FlagAction::None).
    pub fn simple(cycles: u8) -> Self {
        Self {
            cycles,
            z: false,
            h: false,
            c: false,
            n: false,
        }
    }

    /// For instructions where the cycle count can change (e.g., JR NZ, e8).
    /// Pass 'condition_met' to choose between cycles[0] and cycles[1].
    pub fn jump(instr: &OpcodeInfo, condition_met: bool) -> Self {
        let cycles = if condition_met {
            instr.cycles[0]
        } else {
            instr.cycles[1]
        };
        Self::simple(cycles)
    }
}

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
        InstructionResult::from_instr(&self)
    }
    /// For instructions that need to pass calculated flag proposals.
    pub fn result_with_flags(&self, z: bool, n: bool, h: bool, c: bool) -> InstructionResult {
        InstructionResult::with_flags(self, z, n, h, c)
    }
    pub fn last_operand(&self) -> (Target, bool) {
        if self.operands.len() == 1 {
            self.operands[0]
        } else {
            self.operands[1]
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FlagAction {
    None,      // "-" (Not affected)
    Set,       // "1" (Always set)
    Reset,     // "0" (Always reset)
    Calculate, // "Z", "N", "H", or "C" (Computed at runtime)
    Invert,    // Added for CCF (Complement Carry Flag)
}

#[derive(Debug, Copy, Clone)]
pub struct FlagSpec {
    pub z: FlagAction,
    pub n: FlagAction,
    pub h: FlagAction,
    pub c: FlagAction,
}
impl fmt::Display for FlagAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let c = match self {
            FlagAction::None => '-',      // Not affected
            FlagAction::Calculate => 'v', // Varies/Calculated (or use 'Z','N', etc)
            FlagAction::Set => '1',       // Hardcoded Set
            FlagAction::Reset => '0',     // Hardcoded Reset
            FlagAction::Invert => '!',
        };
        write!(f, "{}", c)
    }
}

impl fmt::Display for FlagSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We use the standard ZNHC order
        // Using 'v' for Calculate, but you can override it for specific positions
        let z = if self.z == FlagAction::Calculate {
            'Z'
        } else {
            format!("{}", self.z).chars().next().unwrap()
        };
        let n = if self.n == FlagAction::Calculate {
            'N'
        } else {
            format!("{}", self.n).chars().next().unwrap()
        };
        let h = if self.h == FlagAction::Calculate {
            'H'
        } else {
            format!("{}", self.h).chars().next().unwrap()
        };
        let c = if self.c == FlagAction::Calculate {
            'C'
        } else {
            format!("{}", self.c).chars().next().unwrap()
        };

        write!(f, "[{}{}{}{}]", z, n, h, c)
    }
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
            write!(
                f,
                " {: <60} {: >10}",
                operand_strings.join(", "),
                self.flags
            )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cb_opcode_decoding() {
        // We will test a few key instructions to ensure the logic is sound
        let cases = vec![
            // (Opcode, Mnemonic)
            (0x4E, Mnemonic::BIT),
            (0xDE, Mnemonic::SET),
        ];

        for (opcode, expected_name) in cases {
            let info = CB_OPCODES[opcode].unwrap();

            assert_eq!(
                info.mnemonic, expected_name,
                "Mnemonic mismatch for opcode {:02X}",
                opcode
            );
        }
    }

    #[test]
    fn verify_jr_gets_carry_instead_of_c_registry() {
        let info = OPCODES[0x38].unwrap();
        let carry_target = info.operands[0].0;
        assert_eq!(carry_target, Target::Condition(Condition::Carry));
    }
}
