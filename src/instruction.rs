#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Target {
    Register8(Reg8),
    Register16(Reg16),
    Immediate8,
    Immediate16,
    AddrImmediate16,
    AddrImmediate8, // for LDH (a8)
    AddrRegister16(Reg16),
    Bit(u8),
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

// THE TRICK: Include the generated code right here.
// The generated code will "see" Target, Reg8, etc. because they are in scope.
include!(concat!(env!("OUT_DIR"), "/opcodes_generated.rs"));
