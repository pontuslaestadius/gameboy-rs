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
    flags: HashMap<String, String>,
}

#[derive(Deserialize)]
struct RawOperand {
    name: String,
    decrement: Option<bool>,
    increment: Option<bool>,
    immediate: bool,
}

fn generate_rom_tests() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let destination = Path::new(&out_dir).join("generated_rom_tests.rs");

    let mut test_code = String::new();
    let roms = glob::glob("tests/tools/**/*.gb").expect("Failed to read glob pattern");

    for entry in roms.filter_map(Result::ok) {
        let path = entry.to_str().unwrap();
        let name = entry
            .to_str()
            .unwrap()
            .replace("/", "_")
            .replace(")", "_")
            .replace("(", "_")
            .replace(",", "_")
            .replace(" ", "_")
            .replace("-", "_")
            .replace(".", "_")
            .replace("__", "_"); // Fixes a linting issue in naming after previous replacements.

        // Reduce name length for simplicity.
        let name = name
            .strip_prefix("tests_tools_gb_test_roms_")
            .unwrap_or(&name);

        // This test is a bit annoying to cover, we have other coverage for it.
        // It won't finish on time out or other issues.
        // Sound tests are covered by the individual test cases.
        if name.contains("cpu_instrs_gb")
            || name.contains("dmg_sound_dmg_sound_gb")
            || name.contains("cgb_sound_cgb_sound_gb")
            || name.contains("oam_bug_oam_bug_gb")
        {
            continue;
        }

        test_code.push_str(&format!(
            "#[test] fn {}() {{ run_test(r#\"{}\"#); }}\n",
            name, path
        ));
    }

    fs::write(destination, test_code).unwrap();
}

fn map_target(operand: &RawOperand, op_code: u8) -> String {
    // 1. Handle specialized Bit targets first
    if let Ok(bit) = operand.name.parse::<u8>() {
        return format!("Target::Bit({})", bit);
    }

    // --- RST Vectors ---
    // These usually start with '$' in your JSON (e.g., $00, $08, $10...)
    if operand.name.starts_with('$') {
        let hex = operand.name.trim_start_matches('$');
        let val = u8::from_str_radix(hex, 16).unwrap_or(0);
        return format!("Target::Vector(0x{:02X})", val);
    }

    let name = operand.name.as_str();
    // These specific Opcode ranges ONLY use "C" as a Condition
    let is_branch_opcode = match op_code {
        0x20 | 0x28 | 0x30 | 0x38 => true, // JR NZ, Z, NC, C
        0xC0 | 0xC8 | 0xD0 | 0xD8 => true, // RET NZ, Z, NC, C
        0xC2 | 0xCA | 0xD2 | 0xDA => true, // JP NZ, Z, NC, C
        0xC4 | 0xCC | 0xD4 | 0xDC => true, // CALL NZ, Z, NC, C
        _ => false,
    };

    if is_branch_opcode && name == "C" {
        return "Target::Condition(Condition::Carry)".into();
    }

    // Also handle the other conditions for these opcodes
    if is_branch_opcode {
        match name {
            "NZ" => return "Target::Condition(Condition::NotZero)".into(),
            "Z" => return "Target::Condition(Condition::Zero)".into(),
            "NC" => return "Target::Condition(Condition::NotCarry)".into(),
            _ => {}
        }
    }

    match (
        operand.name.as_str(),
        operand.immediate,
        operand.increment.unwrap_or(false),
        operand.decrement.unwrap_or(false),
    ) {
        // --- Jump Conditions (Handle these early!) ---
        // ("NZ", _, _, _) => "Target::Condition(Condition::NotZero)".into(),
        // ("Z", _, _, _) => "Target::Condition(Condition::Zero)".into(),
        // ("NC", _, _, _) => "Target::Condition(Condition::NotCarry)".into(),

        // The "C" Ambiguity Fix:
        // In JR C, e8 or CALL C, n16, 'C' is a flag condition.
        // In those cases, 'immediate' is often false in the JSON,
        // whereas 'LD A, C' has 'C' as immediate: true.
        // ("C", false, false, false) => "Target::Condition(Condition::Carry)".into(),

        // --- Standard Direct Register Access ---
        ("A", true, _, _) => "Target::Register8(Reg8::A)".into(),
        ("B", true, _, _) => "Target::Register8(Reg8::B)".into(),
        ("C", true, _, _) => "Target::Register8(Reg8::C)".into(), // Register C

        // --- 16-bit Pointers with Side Effects ---
        ("HL", false, true, false) => "Target::AddrRegister16Increment(Reg16::HL)".into(),
        ("HL", false, false, true) => "Target::AddrRegister16Decrement(Reg16::HL)".into(),

        // --- Standard Indirect Addressing (Memory Pointers) ---
        // immediate: false usually means it's wrapped in parentheses (HL), (BC), etc.
        ("HL", false, false, false) => "Target::AddrRegister16(Reg16::HL)".into(),
        ("BC", false, _, _) => "Target::AddrRegister16(Reg16::BC)".into(),
        ("DE", false, _, _) => "Target::AddrRegister16(Reg16::DE)".into(),
        ("C", false, _, _) => "Target::AddrRegister8(Reg8::C)".into(), // For LDH A, (C)

        // --- Standard Direct Register Access ---
        // ("A", true, _, _) => "Target::Register8(Reg8::A)".into(),
        // ("B", true, _, _) => "Target::Register8(Reg8::B)".into(),
        // ("C", true, _, _) => "Target::Register8(Reg8::C)".into(),
        ("D", true, _, _) => "Target::Register8(Reg8::D)".into(),
        ("E", true, _, _) => "Target::Register8(Reg8::E)".into(),
        ("H", true, _, _) => "Target::Register8(Reg8::H)".into(),
        ("L", true, _, _) => "Target::Register8(Reg8::L)".into(),

        ("AF", true, _, _) => "Target::Register16(Reg16::AF)".into(),
        ("BC", true, _, _) => "Target::Register16(Reg16::BC)".into(),
        ("DE", true, _, _) => "Target::Register16(Reg16::DE)".into(),
        ("HL", true, _, _) => "Target::Register16(Reg16::HL)".into(),
        ("SP", _, _, _) => "Target::StackPointer".into(),

        // --- Literals and Absolute Addresses ---
        ("n8", _, _, _) => "Target::Immediate8".into(),
        ("n16", _, _, _) => "Target::Immediate16".into(),
        ("a8", _, _, _) => "Target::AddrImmediate8".into(),
        ("a16", _, _, _) => "Target::AddrImmediate16".into(),
        // --- Jump Conditions ---
        // ("NZ", _, _, _) => "Target::Condition(Condition::NotZero)".into(),
        // ("Z", _, _, _) => "Target::Condition(Condition::Zero)".into(),
        // ("NC", _, _, _) => "Target::Condition(Condition::NotCarry)".into(),
        // ("C", _, _, _) if operand.immediate => "Target::Condition(Condition::Carry)".into(),

        // --- Relative Address Offset ---
        // 'e8' is a signed 8-bit displacement used in JR (Jump Relative)
        ("e8", _, _, _) => "Target::Relative8".into(),

        // --- Fallback/Error Catching ---
        _ => {
            println!(
                "cargo:warning=Unknown target combo: name={}, imm={}, inc={:?}, dec={:?}",
                operand.name, operand.immediate, operand.increment, operand.decrement
            );
            "Target::Immediate8".into()
        }
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
    code.push_str("pub fn dispatch(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult {");
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
        code.push_str(&format!(
            "    fn {}(&mut self, instr: OpcodeInfo, bus: &mut impl Memory) -> InstructionResult;\n",
            mnemonic.to_lowercase()
        ));
    }
    code.push_str("}\n");
    code
}

fn map_flag_action(action: &str) -> String {
    match action {
        "Z" | "N" | "H" | "C" => "FlagAction::Calculate".to_string(),
        "0" => "FlagAction::Reset".to_string(),
        "1" => "FlagAction::Set".to_string(),
        "-" => "FlagAction::None".to_string(),
        _ => panic!("Unknown flag action in JSON: {}", action),
    }
}

fn main() {
    let json_str =
        fs::read_to_string("src/opcodes/data/opcodes.json").expect("Missing opcodes.json");
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
        for i in 0..=255 {
            let key = format!("0x{:02X}", i);
            if let Some(op) = table.get(&key) {
                if op.mnemonic == "PREFIX" || op.mnemonic.starts_with("ILLEGAL") {
                    code.push_str("    None,\n");
                    continue;
                }
                let mut flag_str = String::new();
                for (key, val) in op.flags.iter() {
                    let content = if op.mnemonic == "CCF" && val == "C" {
                        "FlagAction::Invert".to_string()
                    } else {
                        map_flag_action(val)
                    };

                    flag_str.push_str(&format!("{}: {}, ", key.to_ascii_lowercase(), content));
                }
                // TODO: maybe handle flags here? So all static flags, e.g.
                // forced without the operation results mattering.
                let mut ops_str = String::new();
                unique_mnemonics.insert(op.mnemonic.clone());
                for o in &op.operands {
                    ops_str.push_str(&format!("({}, {}),", map_target(o, i), o.immediate));
                }

                let bit_index = if name == "CB_OPCODES" {
                    (i >> 3) & 0x07
                } else {
                    0
                };
                code.push_str(&format!(
                    "    Some(OpcodeInfo {{ mnemonic: Mnemonic::{}, bytes: {}, cycles: &{:?}, operands: &[{}], bit_index: {}, flags: FlagSpec {{{}}}  }}),\n",
                    op.mnemonic, op.bytes, op.cycles, ops_str, bit_index, flag_str
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

    generate_rom_tests();
    println!("wrote generated opcodes to: {:?}", dest_path);
    println!("cargo:rerun-if-changed=src/opcodes/data/opcodes.json");
    println!("cargo:rerun-if-changed=src/instruction.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
