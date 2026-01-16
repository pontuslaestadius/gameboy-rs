use serde::Deserialize;
use std::collections::HashMap;

use crate::cpu::opcode::{flags::Flags, mnemonic::Mnemonic};

#[derive(Deserialize)]
pub struct Root {
    pub unprefixed: HashMap<String, JsonOpcode>,
    pub cbprefixed: HashMap<String, JsonOpcode>,
}

#[derive(Deserialize, Debug)]
pub struct JsonOpcode {
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
