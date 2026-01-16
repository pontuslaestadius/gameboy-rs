use serde::Deserialize;

use crate::cpu::opcode::{flag_effect::FlagEffect, flags::Flags};

#[derive(Deserialize, Copy, Clone, Debug)]
pub struct Opcode {
    pub mnemonic: &'static str,
    pub bytes: u8,
    pub cycles: [u8; 2],
    pub immediate: bool,
    pub flags: Flags,
}

impl Opcode {
    pub const INVALID: Opcode = Opcode {
        mnemonic: "INVALID",
        bytes: 1,
        cycles: [0, 0],
        immediate: false,
        flags: Flags {
            z: FlagEffect::Untouched,
            n: FlagEffect::Untouched,
            h: FlagEffect::Untouched,
            c: FlagEffect::Untouched,
        },
    };
}
