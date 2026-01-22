use super::*;

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
