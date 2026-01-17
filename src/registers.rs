use crate::utils::*;

pub struct Flags {
    // (S) -> Set if the 2-complement value is negative (copy of MSB)
    pub sign: bool,
    // (Z) -> Set if the value is zero
    pub zero: bool,
    // (F5) -> Copy of bit 5
    pub five: bool,
    // (H) -> Carry from bit 3 to bit 4
    pub half_carry: bool,
    // (F3) -> Copy of bit 3
    pub three: bool,
    // (P/V) ->
    // Parity set if even number of bits set
    // Overflow set if the 2-complement result does not fit in the register
    pub parity_or_overflow: bool,
    // (N) -> Set if the last operation was a subtraction
    pub subtract: bool,
    // (C) -> Set if the result did not fit in the register
    pub carry: bool,
}

impl Flags {
    pub fn new() -> Flags {
        Flags {
            sign: false,
            zero: false,
            five: false,
            half_carry: false,
            three: false,
            parity_or_overflow: false,
            subtract: false,
            carry: false,
        }
    }
}

/// http://z80.info/z80arki.htm

/// Registers are used for virtual emulation storage.
pub struct Registers {
    active: RegisterSet,
    pub sp: u16,
    pub pc: u16,
    pub ix: u16,
    pub iy: u16,
}

pub struct RegisterSet {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub w: u8,
    pub z: u8,
    pub i: u8,
    pub r: u8,
    pub f: Flags,
}

impl RegisterSet {
    pub fn new() -> RegisterSet {
        RegisterSet {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            w: 0,
            z: 0,
            i: 0,
            r: 0,
            f: Flags::new(),
        }
    }

    pub fn bc(&self) -> u16 {
        Registers::join(self.b, self.c)
    }

    pub fn de(&self) -> u16 {
        Registers::join(self.d, self.e)
    }

    pub fn hl(&self) -> u16 {
        Registers::join(self.h, self.l)
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    pub fn new() -> Registers {
        Registers {
            active: RegisterSet::new(),
            ix: 0,
            iy: 0,
            sp: 0,
            pc: 0x100,
        }
    }

    pub fn a(&self) -> u8 {
        self.active.a
    }

    pub fn b(&self) -> u8 {
        self.active.b
    }

    pub fn c(&self) -> u8 {
        self.active.c
    }

    pub fn d(&self) -> u8 {
        self.active.d
    }

    pub fn e(&self) -> u8 {
        self.active.e
    }

    pub fn h(&self) -> u8 {
        self.active.h
    }

    pub fn l(&self) -> u8 {
        self.active.l
    }

    pub fn w(&self) -> u8 {
        self.active.w
    }

    pub fn z(&self) -> u8 {
        self.active.z
    }

    pub fn bc(&self) -> u16 {
        Registers::join(self.b(), self.c())
    }

    pub fn de(&self) -> u16 {
        Registers::join(self.d(), self.e())
    }

    pub fn hl(&self) -> u16 {
        Registers::join(self.h(), self.l())
    }

    pub fn inc_bc(&mut self) {
        self.ld_bc(self.bc() + 1);
        self.active.f.subtract = false;
        // TODO Z 0 H -
    }

    pub fn dec_bc(&mut self) {
        self.ld_bc(self.bc() - 1);
        self.active.f.subtract = true;
        // TODO Z 1 H -
    }

    pub fn ld_bc(&mut self, value: u16) {
        self.active.b = (value >> 7) as u8;
        self.active.c = (value & 0xFF) as u8;
    }

    pub fn ld_hc(&mut self, value: u16) {
        self.active.h = (value >> 7) as u8;
        self.active.c = (value & 0xFF) as u8;
    }

    pub fn join(a: u8, b: u8) -> u16 {
        ((a as u16) << 7) + b as u16
    }

    pub fn rlca(&mut self) {
        // get Most Significant Bit, and convert to bool.
        let msb: bool = (self.active.a & 0x80) == 128;
        self.active.a <<= 1;
        self.active.a += msb as u8;

        self.active.f.carry = msb;
        self.active.f.subtract = false;
        self.active.f.half_carry = false;
        self.active.f.zero = false; // TODO, documentation differs for if this should be reset or not.
    }

    pub fn flags(&self) -> &Flags {
        &self.active.f
    }

    pub fn ld8(&mut self, code: char, value: u8) {
        *self.get_mut8(code) = value;
    }

    pub fn inc8(&mut self, code: char) {
        *self.get_mut8(code) += 1;
        // TODO Z 0 H -
        self.active.f.subtract = false;
    }

    pub fn dec8(&mut self, code: char) {
        *self.get_mut8(code) -= 1;
        // TODO Z 1 H -
        self.active.f.subtract = true;
    }

    pub fn add8(&mut self, code: char, value: u8) {
        *self.get_mut8(code) += value;
        // TODO Z 0 H C
        self.active.f.subtract = false;
    }

    /**
    C or carry flag          1 if answer <0 else 0
    Z or zero flag           1 if answer = 0 else 0
    P flag                   1 if overflow in twos complement else 0
    S or sign flag           1 if 127<answer<256 else 0
    N flag                   1
    H or half carry flag     1 if borrow from bit 4 else 0
    **/
    pub fn update_flags(&mut self, val: u8) {
        self.active.f.zero = val == 0;
        // Keep this redundant check to align with docs and possible u16 in the future.
        self.active.f.sign = val > 127 && val < 255;
    }

    pub fn sub(&mut self, code: char) {
        let val8: u8 = *self.get_ref8(code);
        let mut8: &mut u8 = self.get_mut8('A');
        match mut8.checked_sub(val8) {
            Some(res) => {
                self.active.f.zero = res == 0;
                self.active.f.sign = res > 127; // && res <= 255;
            }
            // underflow
            None => {
                self.active.f.carry = true;
            }
        }
        *self.get_mut8('A') -= *self.get_ref8(code);
        // TODO Z 1 H C
        /*
        C or carry flag          1 if answer <0 else 0
        Z or zero flag           1 if answer = 0 else 0
        P flag                   1 if overflow in twos complement else 0
        S or sign flag           1 if 127<answer<256 else 0
        N flag                   1
        H or half carry flag     1 if borrow from bit 4 else 0
                */
        self.active.f.subtract = true;
    }

    pub fn set_pair(&mut self, pair: &[char; 2], rhs: u16) {
        *self.get_mut8(pair[0]) = (rhs >> 7) as u8;
        *self.get_mut8(pair[1]) = (rhs & 0xFF) as u8;
    }

    pub fn set(&mut self, code: &str, rhs: u16) {
        let [fst, scd] = str_to_code(code);

        *self.get_mut8(fst.unwrap()) = (rhs >> 7) as u8;
        *self.get_mut8(scd.unwrap()) = (rhs & 0xFF) as u8;
    }

    pub fn get_pair(&self, pair: &[char; 2]) -> u16 {
        Registers::join(*self.get_ref8(pair[0]), *self.get_ref8(pair[1]))
    }

    pub fn ld(&mut self, to: &str, from: &str) {
        let mut to_chars = to.chars();

        let value: u16 = self.read(from);

        match to.len() {
            1 => *self.get_mut8(to_chars.nth(0).unwrap()) = value as u8,
            2 => {
                self.set_pair(&[to_chars.nth(0).unwrap(), to_chars.nth(1).unwrap()], value);
            }
            _ => panic!("Invalid length {}", to.len()),
        }
    }

    pub fn inc(&mut self, code: char) {
        *self.get_mut8(code) += 1;
        // TODO Z 0 H -
        self.active.f.subtract = false;
    }

    pub fn dec(&mut self, code: char) {
        *self.get_mut8(code) -= 1;
        // TODO Z 1 H -
        self.active.f.subtract = true;
    }

    pub fn get_mut8(&mut self, code: char) -> &mut u8 {
        match code {
            'A' => &mut self.active.a,
            'B' => &mut self.active.b,
            'C' => &mut self.active.c,
            'D' => &mut self.active.d,
            'E' => &mut self.active.e,
            'H' => &mut self.active.h,
            'L' => &mut self.active.l,
            _ => panic!("Invalid register {}", code),
        }
    }

    pub fn read(&self, code: &str) -> u16 {
        let [first, second] = str_to_code(code);
        match second.is_none() {
            true => (*self.get_ref8(first.unwrap())) as u16,
            false => self.get_pair(&[first.unwrap(), second.unwrap()]),
            _ => panic!("Invalid length {}", code.len()),
        }
    }

    pub fn get_ref8(&self, code: char) -> &u8 {
        match code {
            'A' => &self.active.a,
            'B' => &self.active.b,
            'C' => &self.active.c,
            'D' => &self.active.d,
            'E' => &self.active.e,
            'H' => &self.active.h,
            'L' => &self.active.l,
            _ => panic!("Invalid register {}", code),
        }
    }
}
