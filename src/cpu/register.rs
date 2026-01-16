use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,

    AF,
    BC,
    DE,
    HL,

    SP,
    PC,
}
