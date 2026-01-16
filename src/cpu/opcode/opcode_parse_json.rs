use crate::cpu::opcode::opcode_loader::Root;

use super::helpers::*;
use super::opcode::*;
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

    // let unprefixed = [Opcode::INVALID; 256];
    // let cbprefixed = [Opcode::INVALID; 256];

    // println!("{:?}", root.unprefixed);
    // for (key, raw) in root.unprefixed.into_iter() {
    // println!("{}", raw.mnemonic);
    //     let byte = parse_opcode_byte(&key);

    //     unprefixed[byte as usize] = Opcode {
    //         cycles: parse_cycles(&raw.cycles),
    //     };
    // }

    // for (key, raw) in root.cb.into_iter() {
    // println!("{}", raw.mnemonic);
    //     let byte = parse_opcode_byte(&key);

    //     cbprefixed[byte as usize] = Opcode {
    //         mnemonic: Box::leak(raw.mnemonic.into_boxed_str()),
    //         bytes: raw.bytes,
    //         cycles: parse_cycles(&raw.cycles),
    //         immediate: raw.immediate,
    //         flags: Flags {
    //             z: parse_flag(&raw.flags.Z),
    //             n: parse_flag(&raw.flags.N),
    //             h: parse_flag(&raw.flags.H),
    //             c: parse_flag(&raw.flags.C),
    //         },
    //     };
    // }

    // OpcodeTables {
    // unprefixed,
    // cbprefixed,
    // }
}
