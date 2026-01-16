use serde::{Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[allow(non_camel_case_types)]
pub enum Mnemonic {
    ADC,
    ADD,
    AND,
    BIT,
    CALL,
    CCF,
    CP,
    CPL,
    DAA,
    DEC,
    DI,
    EI,
    HALT,
    ILLEGAL_D3,
    ILLEGAL_DB,
    ILLEGAL_DD,
    ILLEGAL_E3,
    ILLEGAL_E4,
    ILLEGAL_EB,
    ILLEGAL_EC,
    ILLEGAL_ED,
    ILLEGAL_F4,
    ILLEGAL_FC,
    ILLEGAL_FD,
    INC,
    JP,
    JR,
    LD,
    LDH,
    NOP,
    OR,
    POP,
    PREFIX,
    PUSH,
    RES,
    RET,
    RETI,
    RL,
    RLA,
    RLC,
    RLCA,
    RR,
    RRA,
    RRC,
    RRCA,
    RST,
    SBC,
    SCF,
    SET,
    SLA,
    SRA,
    SRL,
    STOP,
    SUB,
    SWAP,
    XOR,
}

impl<'de> Deserialize<'de> for Mnemonic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Mnemonic::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for Mnemonic {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Mnemonic::*;

        match s {
            "ADC" => Ok(ADC),
            "ADD" => Ok(ADD),
            "AND" => Ok(AND),
            "BIT" => Ok(BIT),
            "CALL" => Ok(CALL),
            "CCF" => Ok(CCF),
            "CP" => Ok(CP),
            "CPL" => Ok(CPL),
            "DAA" => Ok(DAA),
            "DEC" => Ok(DEC),
            "DI" => Ok(DI),
            "EI" => Ok(EI),
            "HALT" => Ok(HALT),

            "ILLEGAL_D3" => Ok(ILLEGAL_D3),
            "ILLEGAL_DB" => Ok(ILLEGAL_DB),
            "ILLEGAL_DD" => Ok(ILLEGAL_DD),
            "ILLEGAL_E3" => Ok(ILLEGAL_E3),
            "ILLEGAL_E4" => Ok(ILLEGAL_E4),
            "ILLEGAL_EB" => Ok(ILLEGAL_EB),
            "ILLEGAL_EC" => Ok(ILLEGAL_EC),
            "ILLEGAL_ED" => Ok(ILLEGAL_ED),
            "ILLEGAL_F4" => Ok(ILLEGAL_F4),
            "ILLEGAL_FC" => Ok(ILLEGAL_FC),
            "ILLEGAL_FD" => Ok(ILLEGAL_FD),

            "INC" => Ok(INC),
            "JP" => Ok(JP),
            "JR" => Ok(JR),
            "LD" => Ok(LD),
            "LDH" => Ok(LDH),
            "NOP" => Ok(NOP),
            "OR" => Ok(OR),
            "POP" => Ok(POP),
            "PREFIX" => Ok(PREFIX),
            "PUSH" => Ok(PUSH),
            "RES" => Ok(RES),
            "RET" => Ok(RET),
            "RETI" => Ok(RETI),
            "RL" => Ok(RL),
            "RLA" => Ok(RLA),
            "RLC" => Ok(RLC),
            "RLCA" => Ok(RLCA),
            "RR" => Ok(RR),
            "RRA" => Ok(RRA),
            "RRC" => Ok(RRC),
            "RRCA" => Ok(RRCA),
            "RST" => Ok(RST),
            "SBC" => Ok(SBC),
            "SCF" => Ok(SCF),
            "SET" => Ok(SET),
            "SLA" => Ok(SLA),
            "SRA" => Ok(SRA),
            "SRL" => Ok(SRL),
            "STOP" => Ok(STOP),
            "SUB" => Ok(SUB),
            "SWAP" => Ok(SWAP),
            "XOR" => Ok(XOR),

            _ => Err("unknown mnemonic"),
        }
    }
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
