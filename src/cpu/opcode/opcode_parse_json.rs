use super::opcode::*;
use crate::cpu::opcode::opcode_loader::Root;
use log::info;

pub struct OpcodeTables {
    pub unprefixed: [Opcode; 256],
    pub cbprefixed: [Opcode; 256],
}

pub fn load_Json() -> Root {
    let path = "data/opcodes.json";
    info!("Loading Opcode specifications from '{}'", path);
    let json = std::fs::read_to_string(path).expect("Missing opcode data");

    load_opcodes(&json)
}

pub fn load_opcodes(json: &str) -> Root {
    let root: Root = serde_json::from_str(json).expect("Invalid opcode JSON");
    root
}
