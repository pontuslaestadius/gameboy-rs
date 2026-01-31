pub trait Alu {
    // Arithmetic
    fn alu_add(a: u8, b: u8) -> Self;
    fn alu_adc(a: u8, b: u8, carry: bool) -> Self;
    fn alu_sub(a: u8, b: u8) -> Self;
    fn alu_sbc(a: u8, b: u8, carry: bool) -> Self;

    // Logical Operations
    fn alu_and(a: u8, b: u8) -> Self;
    fn alu_or(a: u8, b: u8) -> Self;
    fn alu_xor(a: u8, b: u8) -> Self;

    // Unit Operations
    fn alu_inc(val: u8) -> Self;
    fn alu_dec(val: u8) -> Self;
}
