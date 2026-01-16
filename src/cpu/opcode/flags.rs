use crate::cpu::opcode::flag_effect::FlagEffect;
use serde::Deserialize;

/// Flag documentation gathered from:
/// http://z80.info/z80sflag.htm
/// And has only been stylized but with identical information.
#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Flags {
    #[serde(rename = "Z")]
    pub z: FlagEffect,
    #[serde(rename = "N")]
    pub n: FlagEffect,
    #[serde(rename = "H")]
    pub h: FlagEffect,
    #[serde(rename = "C")]
    pub c: FlagEffect,
}

impl Flags {}
