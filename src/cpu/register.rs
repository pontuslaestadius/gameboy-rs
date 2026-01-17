#[derive(Copy, Clone, Debug)]
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
