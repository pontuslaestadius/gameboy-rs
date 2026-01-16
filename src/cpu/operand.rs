use serde::Deserialize;

use crate::cpu::{immediate::Immediate, register::Register};

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum Operand {
    Immediate(Immediate),
    Register(Register),
    Indirect(Register), // (HL), (BC), (DE)
    IndirectFF00C,      // (FF00 + C)
}
