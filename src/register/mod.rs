use super::Registers;
use super::Flags;




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

}

impl Flags {

    // Private to the module.
    pub fn new() -> Flags {
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