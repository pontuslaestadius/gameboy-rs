use std::fmt;

/// Holds an 8-bit binary.
/// Values are stored as booleans because they hold the lowest amount of data in memory.
#[derive(PartialEq)]
pub struct SmartBinary {
    pub zer: bool,
    pub one: bool,
    pub two: bool,
    pub thr: bool,
    pub fou: bool,
    pub fiv: bool,
    pub six: bool,
    pub sev: bool,
}

impl fmt::Debug for SmartBinary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let octal_values: String = self
            .x_y_z_p_q()
            .iter()
            .map(|x| format!("{}", x))
            .collect::<String>();
        write!(f, "{} {} -> {:?}", self.hex(), self.binary(), octal_values)
    }
}

impl SmartBinary {
    pub fn new(byte: u8) -> SmartBinary {
        // Formats it from a byte to a binary.
        let bytes = format!("{:b}", byte);

        // If it is less than 8bit length, add trailing zeros.
        let formatted = if bytes.len() != 8 {
            let mut extra = String::new();
            for _ in bytes.len()..8 {
                extra.push('0');
            }
            extra.push_str(bytes.as_str());
            extra
        } else {
            bytes
        };

        let mut formatted_chars = formatted.chars();
        let convert_u8b = |x| x == '1';

        // nth consumes the elements, so calling 0 on each one returns different elements:
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.nth
        SmartBinary {
            zer: convert_u8b(formatted_chars.nth(0).unwrap()),
            one: convert_u8b(formatted_chars.nth(0).unwrap()),
            two: convert_u8b(formatted_chars.nth(0).unwrap()),
            thr: convert_u8b(formatted_chars.nth(0).unwrap()),
            fou: convert_u8b(formatted_chars.nth(0).unwrap()),
            fiv: convert_u8b(formatted_chars.nth(0).unwrap()),
            six: convert_u8b(formatted_chars.nth(0).unwrap()),
            sev: convert_u8b(formatted_chars.nth(0).unwrap()),
        }
    }

    pub fn binary(&self) -> String {
        self
            .as_list()
            .iter()
            .map(|x| format!("{}", x))
            .collect::<String>()
    }

    pub fn hex(&self) -> String {
        let n: u32 = u32::from_str_radix(&self.binary(), 2).unwrap();
        format!("{:x}", n)
    }

    /// Creates a smartbinary from a list.
    pub fn from_list(list: [u8; 8]) -> SmartBinary {
        let convert_u8b = |x| {
            if x == 1 {
                true
            } else {
                false
            }
        };

        SmartBinary {
            zer: convert_u8b(list[0]),
            one: convert_u8b(list[1]),
            two: convert_u8b(list[2]),
            thr: convert_u8b(list[3]),
            fou: convert_u8b(list[4]),
            fiv: convert_u8b(list[5]),
            six: convert_u8b(list[6]),
            sev: convert_u8b(list[7]),
        }
    }

    /// Returns a binary list of a SmartBinary.
    pub fn as_list(&self) -> [u8; 8] {
        let convert = |x| {
            if x {
                1
            } else {
                0
            }
        };

        [
            convert(self.zer),
            convert(self.one),
            convert(self.two),
            convert(self.thr),
            convert(self.fou),
            convert(self.fiv),
            convert(self.six),
            convert(self.sev),
        ]
    }

    /// make a flipped list.
    pub fn as_list_flipped(&self) -> [u8; 8] {
        let list = self.as_list();
        // Turns 1 to 0 and 0 to 1.
        let flip = |x| {
            if x == 1 {
                0
            } else {
                1
            }
        };

        [
            flip(list[0]),
            flip(list[1]),
            flip(list[2]),
            flip(list[3]),
            flip(list[4]),
            flip(list[5]),
            flip(list[6]),
            flip(list[7]),
        ]
    }

    /// Returns the SmartBinary as a u8.
    pub fn as_u8(&self) -> u8 {
        panic!("TODO u8")
    }

    // Will weight self over other.
    pub fn as_u16(&self, _other: &SmartBinary) -> u8 {
        panic!("TODO u16");
    }

    /// Converts it to an octal. Using two's complement:
    /// https://en.wikipedia.org/wiki/Two%27s_complement
    pub fn as_i8(&self) -> i8 {
        // Get the list if bits.
        let mut list = self.as_list();
        let mut neg = 1;

        // If it is a negative or not.
        if list[0] == 1 {
            // Flip the listf from 1 to 0 and 0 to 1.
            list = self.as_list_flipped();
            // Add one to it.
            // Ignore sign flag.
            for ind in 1..=list.len() {
                // Start from the end of the list with LSB:
                let i = list.len() - ind;

                // If it's a 1 we change it to 0 and carry it to the next.
                // If it is a 0 we set the value and finish.
                if list[i] == 0 {
                    list[i] = 1;
                    break;
                } else {
                    list[i] = 0;
                }
            }
            // Set it to be a negative multiplier.
            neg = -1;
        }

        // Multiple the remaining 7 bits to form a unsigned char.
        let mut multiplier: u32 = 1;
        let mut result: i16 = 0;
        for i in 1..list.len() {
            let i = list.len() - i;
            result += list[i] as i16 * multiplier as i16;
            multiplier = multiplier * 2;
        }

        // Multiply the result with the negative.
        neg * result as i8
    }
}