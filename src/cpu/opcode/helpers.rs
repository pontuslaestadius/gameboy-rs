use crate::cpu::opcode::flag_effect::FlagEffect;

pub fn parse_flag(s: &str) -> FlagEffect {
    let ch: char = s.bytes().nth(0).unwrap() as char;
    FlagEffect::from_char(ch)
}

pub fn parse_cycles(cycles: &[u8]) -> [u8; 2] {
    match cycles.len() {
        1 => [cycles[0], cycles[0]],
        2 => [cycles[0], cycles[1]],
        _ => panic!("Invalid cycle count"),
    }
}

pub fn parse_opcode_byte(key: &str) -> u8 {
    u8::from_str_radix(key.trim_start_matches("0x"), 16).expect("Invalid opcode key")
}
