

/// Private struct, as to only make it generate through the new() method.
pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}


/// Flag documentation gathered from:
/// http://z80.info/z80sflag.htm
/// And has only been stylized but with identical information.
struct Flags {

    // (S) -> Set if the 2-complement value is negative (copy of MSB)
    sign: bool,

    // (Z) -> Set if the value is zero
    zero: bool,

    // (F5) -> Copy of bit 5
    five: bool,

    // (H) -> Carry from bit 3 to bit 4
    half_carry: bool,

    // (F3) -> Copy of bit 3
    three: bool,

    // (P/V) ->
    // Parity set if even number of bits set
    // Overflow set if the 2-complement result does not fit in the register
    parity_or_overflow: bool,

    // (N) -> Set if the last operation was a subtraction
    subtract: bool,

    // (C) -> Set if the result did not fit in the register
    carry: bool
}


impl Registers {

    /// Only way to generate a new Register because the struct is private.
    pub fn new() -> Registers {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }


    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn set_pc(&mut self, new: u16) {
        self.pc = new;
    }


    /// NOT IMPLEMENTED
    pub fn instruction(instruction: String) {

    }
}

impl Flags {

    // Private to the module.
    fn new() -> Flags {
        Flags {
            sign: false,
            zero: false,
            five: false,
            half_carry: false,
            three: false,
            parity_or_overflow: false,
            subtract: false,
            carry: false
        }
    }
}