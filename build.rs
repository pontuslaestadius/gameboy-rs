// build.rs
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::{env, fs, path::Path};

#[derive(Deserialize)]
struct FullJson {
    unprefixed: HashMap<String, RawOpcode>,
    cbprefixed: HashMap<String, RawOpcode>,
}

#[derive(Deserialize)]
struct RawOpcode {
    mnemonic: String,
    bytes: u8,
    cycles: Vec<u8>,
    operands: Vec<RawOperand>,
}

#[derive(Deserialize)]
struct RawOperand {
    name: String,
    immediate: bool,
}

fn map_target(name: &str) -> String {
    match name {
        "A" => "Target::Register8(Reg8::A)".into(),
        "B" => "Target::Register8(Reg8::B)".into(),
        "C" => "Target::Register8(Reg8::C)".into(),
        "BC" => "Target::Register16(Reg16::BC)".into(),
        "HL" => "Target::Register16(Reg16::HL)".into(),
        "n8" => "Target::Immediate8".into(),
        "n16" => "Target::Immediate16".into(),
        "a16" => "Target::AddrImmediate16".into(),
        "a8" => "Target::AddrImmediate8".into(),
        "(HL)" => "Target::AddrRegister16(Reg16::HL)".into(),
        n if n.parse::<u8>().is_ok() => format!("Target::Bit({})", n),
        _ => "Target::Immediate8".into(), // Fallback
    }
}

fn produce_mnemonics_enum(hashset: &HashSet<String>) -> String {
    let mut code = String::new();
    code.push_str("\n#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]\n#[allow(non_camel_case_types)]\npub enum Mnemonic {\n");
    for item in hashset {
        code.push_str(&format!("{item},\n"));
    }
    code.push_str("}\n");
    code
}

fn produce_dispatcher_fn(hashset: &HashSet<String>) -> String {
    let mut code = String::new();
    code.push_str("impl Cpu {\n");
    code.push_str("pub fn dispatch(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8 {");
    // Since we increment PC before this, we decrement it in our log.
    code.push_str("match instr.mnemonic {\n");

    for mnemonic in hashset {
        code.push_str(&format!(
            "Mnemonic::{} => self.{}(instr, bus),\n",
            mnemonic,
            mnemonic.to_lowercase(),
        ));
    }
    code.push_str("}\n}\n}\n");
    code
}

fn produce_mnemonics_coverage_trait(hashset: &HashSet<String>) -> String {
    let mut code = String::new();
    code.push_str("pub trait InstructionSet {\n");

    for mnemonic in hashset {
        // Generates: fn ld(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8;
        code.push_str(&format!(
            "    fn {}(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> u8;\n",
            mnemonic.to_lowercase()
        ));
    }
    code.push_str("}\n");
    code
}

fn main() {
    let json_str = fs::read_to_string("src/data/opcodes.json").expect("Missing opcodes.json");
    let data: FullJson = serde_json::from_str(&json_str).expect("JSON parse error");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("opcodes_generated.rs");
    let mut code = String::new();
    let mut unique_mnemonics = HashSet::new();

    for (name, table) in [
        ("OPCODES", data.unprefixed),
        ("CB_OPCODES", data.cbprefixed),
    ] {
        code.push_str(&format!(
            "pub const {}: [Option<OpcodeInfo>; 256] = [\n",
            name
        ));
        for i in 0..256 {
            let key = format!("0x{:02X}", i);
            if let Some(op) = table.get(&key) {
                if op.mnemonic == "PREFIX" || op.mnemonic.starts_with("ILLEGAL") {
                    code.push_str(&format!("    None,\n",));
                    continue;
                }
                let mut ops_str = String::new();
                unique_mnemonics.insert(op.mnemonic.clone());
                for o in &op.operands {
                    ops_str.push_str(&format!("({}, {}),", map_target(&o.name), o.immediate));
                }
                code.push_str(&format!(
                    "    Some(OpcodeInfo {{ mnemonic: Mnemonic::{}, bytes: {}, cycles: &{:?}, operands: &[{}] }}),\n",
                    op.mnemonic, op.bytes, op.cycles, ops_str
                ));
            } else {
                code.push_str("    None,\n");
            }
        }
        code.push_str("];\n\n");
    }

    code.push_str(&produce_mnemonics_enum(&unique_mnemonics));
    code.push_str(&produce_mnemonics_coverage_trait(&unique_mnemonics));
    code.push_str(&produce_dispatcher_fn(&unique_mnemonics));

    fs::write(&dest_path, code).unwrap();
    println!("wrote generated opcodes to: {:?}", dest_path);
    println!("cargo:rerun-if-changed=src/data/opcodes.json");
    println!("cargo:rerun-if-changed=build.rs");
}
