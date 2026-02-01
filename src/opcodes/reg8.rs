#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Reg8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

impl std::fmt::Display for Reg8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Since the variants are named A, B, C...
        // the Debug implementation {:?} will output "A", "B", etc.
        write!(f, "{:?}", self)
    }
}
