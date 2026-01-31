/// Represents an Arithmic operation, and it's result
/// The purpose is to make the underlying operations pure.
pub struct AluOutput {
    pub value: u8,
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

impl AluOutput {
    pub fn alu_8bit_add(a: u8, b: u8, carry: bool) -> Self {
        let c_in = if carry { 1 } else { 0 };

        let res = (a as u16) + (b as u16) + (c_in as u16);
        let res_u8 = res as u8;

        // Half-Carry: Carry out of bit 3 into bit 4
        // We check if the sum of the lower nibbles exceeds 0xF
        let h_bit = (a & 0x0F) + (b & 0x0F) + c_in > 0x0F;

        AluOutput {
            value: res_u8,
            z: res_u8 == 0,
            n: false,
            h: h_bit,
            c: res > 0xFF,
        }
    }

    pub fn alu_8bit_dec(value: u8) -> Self {
        let res = value.wrapping_sub(1);

        // Flags:
        let z = res == 0;
        let n = true; // Always true for DEC
        // Half-Carry: Set if there was a borrow from bit 4
        // (i.e., the lower nibble was 0x0 before the decrement)
        let h = (value & 0x0F) == 0;
        AluOutput {
            value: res,
            z,
            n,
            h,
            c: false,
        }
    }

    pub fn alu_8bit_sub(a: u8, b: u8, carry: bool) -> Self {
        let c_in = if carry { 1 } else { 0 };

        // Standard subtraction result
        let res = (a as i16) - (b as i16) - (c_in as i16);
        let res_u8 = res as u8;

        // Half-Carry (Half-Borrow): Set if there is no borrow from bit 4.
        // In GB terms: bit 3 of 'a' was less than (bit 3 of 'b' + c_in)
        let h_bit = (a & 0x0F) < (b & 0x0F) + c_in;

        // Carry (Borrow): Set if the result is negative (a borrow from bit 8)
        let c_bit = (a as u16) < (b as u16) + (c_in as u16);

        AluOutput {
            value: res_u8,
            z: res_u8 == 0,
            n: true,
            h: h_bit,
            c: c_bit,
        }
    }
}
