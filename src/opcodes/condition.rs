#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Condition {
    NotZero,
    Zero,
    NotCarry,
    Carry,
}
