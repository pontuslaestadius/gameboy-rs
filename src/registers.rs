use crate::Flags;

use std::fmt;

/// http://z80.info/z80arki.htm

/// Registers are used for virtual emulation storage.
pub struct Registers {
    pub active_registry: RegisterSet,
    pub passive_registry: RegisterSet,
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

fn to_hex<T: Into<u16>>(val: T) -> String {
    format!("{:01$x}", val.into(), 2)
}

// A  CZPSNH  BC   DE   HL   IX   IY  A' CZPSNH' BC'  DE'  HL'  SP
// 06 000000 0000 0000 0000 0000 0000 00 000000 0000 0000 0000 0000
impl fmt::Debug for Registers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "A   CZPSNH   BC   DE   HL   IX   IY   A'   CZPSNH'  BC'  DE'  HL'  SP"
        );
        write!(
            f,
            "{}  {:?}   {}   {}   {}   {}   {}   {}   {:?}   {}   {}   {}   {}",
            to_hex(self.a()),
            self.active_registry.f,
            to_hex(self.bc()),
            to_hex(self.de()),
            to_hex(self.hl()),
            to_hex(self.ix),
            to_hex(self.iy),
            to_hex(self.passive_registry.a),
            self.passive_registry.f,
            to_hex(self.passive_registry.bc()),
            to_hex(self.passive_registry.de()),
            to_hex(self.passive_registry.hl()),
            to_hex(self.sp),
        )
    }
}

impl Registers {
    pub fn new() -> Registers {
        Registers {
            active_registry: RegisterSet::new(),
            passive_registry: RegisterSet::new(),
            ix: 0,
            iy: 0,
            sp: 0,
            pc: 0,
        }
    }

    pub fn a(&self) -> u8 {
        self.active_registry.a
    }

    pub fn b(&self) -> u8 {
        self.active_registry.b
    }

    pub fn c(&self) -> u8 {
        self.active_registry.c
    }

    pub fn d(&self) -> u8 {
        self.active_registry.d
    }

    pub fn e(&self) -> u8 {
        self.active_registry.e
    }

    pub fn h(&self) -> u8 {
        self.active_registry.h
    }

    pub fn l(&self) -> u8 {
        self.active_registry.l
    }

    pub fn w(&self) -> u8 {
        self.active_registry.w
    }

    pub fn z(&self) -> u8 {
        self.active_registry.z
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

    pub fn join(a: u8, b: u8) -> u16 {
        let s = format!("{:b}{:b}", a, b);
        u16::from_str_radix(&s, 2).unwrap()
    }

    /// Fetches the value given a string from the decode template provided here:
    /// http://www.z80.info/decoding.htm
    pub fn fetch(&self, code: &str) -> u16 {
        // If it's a 8-bit or 16-bit fetch.

        let result = match code.len() {
            // This part is easy.
            1 => match code {
                "A" => self.a(),
                "B" => self.b(),
                "C" => self.c(),
                "D" => self.d(),
                "E" => self.e(),
                "H" => self.h(),
                "L" => self.l(),
                _ => panic!("Invalid register"),
            },

            _ => panic!("Invalid fetch length"),
        };

        result as u16
    }
}
