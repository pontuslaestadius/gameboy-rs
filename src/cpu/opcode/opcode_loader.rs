use serde::Deserialize;
use std::collections::HashMap;

use crate::cpu::opcode::{flags::Flags, mnemonic::Mnemonic};

#[derive(Deserialize)]
pub struct Root {
    pub unprefixed: HashMap<String, Opcode>,
    pub cbprefixed: HashMap<String, Opcode>,
}

#[derive(Deserialize, Debug)]
pub struct Opcode {
    pub mnemonic: Mnemonic,
    pub bytes: u8,
    pub cycles: Vec<u8>,
    pub immediate: bool,
    pub flags: Flags,
    pub operands: Vec<Operand>,
}

#[derive(Deserialize, Debug)]
pub struct Operand {
    pub name: String,
    pub bytes: Option<u8>,
    pub immediate: bool,
}

#[derive(Debug, PartialEq)]
pub enum Target {
    Register8(String),
    Register16(String),
    Immediate8,
    Immediate16,
    Address8,
    Address16,
    Bit(u8),
}

impl Operand {
    pub fn to_target(&self) -> Target {
        match self.name.as_str() {
            "n8" => Target::Immediate8,
            "n16" => Target::Immediate16,
            "a8" => Target::Address8,
            "a16" => Target::Address16,
            // Logic for bits (like in SET 7, A)
            n if n.len() == 1 && n.chars().next().unwrap().is_digit(10) => {
                Target::Bit(n.parse().unwrap())
            }
            // Otherwise, it's a register name
            _ => {
                if self.name.len() <= 2 {
                    Target::Register8(self.name.clone()) // Simplification
                } else {
                    Target::Register16(self.name.clone())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_deserialization() {
        let json_data = r#"
        {
            "0xFF": {
                "mnemonic": "SET",
                "bytes": 2,
                "cycles": [8],
                "operands": [
                    { "name": "7", "bytes": 1, "immediate": true },
                    { "name": "A", "immediate": true }
                ],
                "immediate": true,
                "flags": { "Z": "-", "N": "-", "H": "-", "C": "-" }
            }
        }"#;

        let map: HashMap<String, Opcode> = serde_json::from_str(json_data).unwrap();
        let opcode = &map["0xFF"];

        // assert_eq!(opcode.mnemonic, "SET");
        assert_eq!(opcode.operands.len(), 2);

        // Test the conversion logic
        let target_bit = opcode.operands[0].to_target();
        let target_reg = opcode.operands[1].to_target();

        assert_eq!(target_bit, Target::Bit(7));
        assert_eq!(target_reg, Target::Register8("A".to_string()));
    }
}
