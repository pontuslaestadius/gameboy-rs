#![feature(inclusive_range_syntax)]

use instructions::Opcode;


/// Decoding reading material:
/// Theory: http://www.z80.info/decoding.htm
/// op code: http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode
use std::io::prelude::*;
use std::fs::File;
use std::char;
use std::io::Error;
use std::fmt;

use std::fmt::Debug;

use std::io;
use super::super::Session;

// Only works for 8bit binaries.
#[derive(PartialEq)]
pub struct SmartBinary {
    zer: bool,
    one: bool,
    two: bool,
    thr: bool,
    fou: bool,
    fiv: bool,
    six: bool,
    sev: bool,
}

impl fmt::Debug for SmartBinary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f, "SmartBinary: {:?} -> [{:?}]",
               self.as_list(),
               self.x_y_z_p_q()
        )
    }
}


impl SmartBinary {
    pub fn new(byte: u8) -> SmartBinary {
        // TODO

        let bytes = format!("{:b}", byte);

        let formatted = if bytes.len() != 8 {
            let mut extra = String::new();
            for _ in bytes.len()...8  {
                extra.push('0');
            }
            extra.push_str(bytes.as_str());
            extra
        } else {
            bytes
        };

        let mut formatted_chars = formatted.chars();

        let o = |x| x == '1';

        // nth consumes the elements, so calling 0 on each one returns different elements:
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.nth
        SmartBinary {
            zer: o(formatted_chars.nth(0).unwrap()),
            one: o(formatted_chars.nth(0).unwrap()),
            two: o(formatted_chars.nth(0).unwrap()),
            thr: o(formatted_chars.nth(0).unwrap()),
            fou: o(formatted_chars.nth(0).unwrap()),
            fiv: o(formatted_chars.nth(0).unwrap()),
            six: o(formatted_chars.nth(0).unwrap()),
            sev: o(formatted_chars.nth(0).unwrap()),
        }
    }

    pub fn get(&self, bit: u8) -> bool {
        match bit {
            0 => self.zer,
            1 => self.one,
            2 => self.two,
            3 => self.thr,
            4 => self.fou,
            5 => self.fiv,
            6 => self.six,
            7 => self.sev,
            _ => panic!("Invalid bit value: {}", bit)
        }
    }

    pub fn as_list(&self) -> [u8; 8] {
        let ft = |x| {
            if x {
                1
            } else {
                0
            }
        };

        [
            ft(self.zer),
            ft(self.one),
            ft(self.two),
            ft(self.thr),
            ft(self.fou),
            ft(self.fiv),
            ft(self.six),
            ft(self.sev)
        ]
    }

    pub fn x_y_z_p_q(&self) -> [u8; 5] {

        let orev = |x: bool| {
            if x {
                1
            } else {
                0
            }
        };

        // x = the opcode's 1st octal digit (i.e. bits 7-6)
        let x = octal_digit_from_binary_list(&[
            orev(self.sev),
            orev(self.six)
        ]);

        // y = the opcode's 2nd octal digit (i.e. bits 5-3)
        let y = octal_digit_from_binary_list(&[
            orev(self.fiv),
            orev(self.fou),
            orev(self.thr)
        ]);

        // z = the opcode's 3rd octal digit (i.e. bits 2-0)
        let z = octal_digit_from_binary_list(&[
            orev(self.two),
            orev(self.one),
            orev(self.zer)
        ]);

        // p = y rightshifted one position (i.e. bits 5-4)
        let p = octal_digit_from_binary_list(&[
            orev(self.fiv),
            orev(self.fou),
        ]);

        // q = y modulo 2 (i.e. bit 3)
        let q = octal_digit_from_binary_list(&[
            orev(self.thr),
        ]);

        [x,y,z,p,q]
    }

}


impl Session {

    pub fn op_codes() {

    }

    pub fn unprefixed_opcodes<'a>(&mut self) -> Opcode {
        let step = self.step();

        let binary: SmartBinary = SmartBinary::new(step.clone());

        // Uses experimental splice patterning.
        let [x,y,z,p,q] = binary.x_y_z_p_q();

        // Any commands we can't read, we use an invalid opcode enum.
        let undefined = || {
            Opcode::INVALID(binary)
        };

        match x {
            0 => {

                match z {

                    0 => {

                        match y {

                            0 => Opcode::NOP,
                            1 => Opcode::EXAF,
                            2 => Opcode::DJNZ(0), // TODO fix proper value

                            _ => undefined(),
                        }
                    },

                    7 => {

                        match y {

                            0 => Opcode::RLCA,
                            1 => Opcode::RRCA,
                            2 => Opcode::RLA,
                            3 => Opcode::RRA,
                            4 => Opcode::DAA,
                            5 => Opcode::CPL,
                            6 => Opcode::SCF,
                            7 => Opcode::CCF,

                            _ => undefined(),
                        }
                    },

                    _ => undefined(),
                }
            }

            1 => {

                match y {

                    6 => Opcode::HALT,

                    _ => undefined(),
                }
            }

            3 => {

                match z {

                    1 => {

                        match q {

                            1 => {

                                match p {

                                    0 => Opcode::RET,
                                    1 => Opcode::EXX,
                                    2 => Opcode::JPHL, // TODO fix, should be JP(HL)
                                    3 => Opcode::LDSPHL,

                                    _ => undefined(),

                                }
                            }

                            _ => undefined(),
                        }
                    }

                    3 => {

                        match y {

                            0 => Opcode::JP(0), // TODO fix

                            _ => undefined(),
                        }
                    }

                    7 => {
                        Opcode::RST(y*8)
                    }

                    _ => undefined(),
                }
            }

            _ => undefined(),
        }
    }

    pub fn more_bytes_as_octal(&mut self, nr_bytes: usize) -> u16 {
        let mut vec: Vec<u8> = Vec::new();

        for _ in 0...nr_bytes {
            vec.push(self.step().clone())
        }

        let mut vec_smart_binaries: Vec<SmartBinary> = Vec::new();

        for item in vec.iter() {
            vec_smart_binaries.push(SmartBinary::new(item.clone()));
        }


        // This part only works for 2 or 1 byte.

        if vec_smart_binaries.len() > 1 {
            // Join the lists.

            // two bytes.
            let mut list: [u8; 16] = [
                0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0];

            let list1: &SmartBinary = vec_smart_binaries.get(0).unwrap();
            let list2: &SmartBinary = vec_smart_binaries.get(1).unwrap();

            let orev = |x: bool| {
                if x {
                    1
                } else {
                    0
                }
            };


            list[0] = orev(list1.zer);
            list[1] = orev(list1.one);
            list[2] = orev(list1.two);
            list[3] = orev(list1.thr);
            list[4] = orev(list1.fou);
            list[5] = orev(list1.fiv);
            list[6] = orev(list1.six);
            list[7] = orev(list1.sev);

            list[0+8] = orev(list2.zer);
            list[1+8] = orev(list2.one);
            list[2+8] = orev(list2.two);
            list[3+8] = orev(list2.thr);
            list[4+8] = orev(list2.fou);
            list[5+8] = orev(list2.fiv);
            list[6+8] = orev(list2.six);
            list[7+8] = orev(list2.sev);

            octal_digit_from_binary_list_u16(&list)

        } else {

            let list1: [u8; 8] = vec_smart_binaries.get(0).unwrap().as_list();

            octal_digit_from_binary_list_u16(&list1)
        }
    }

}


pub fn prefix_table(byte: u8) {

}

pub fn opcode_table(byte: u8) {

}


pub fn octal_digit_from_binary_list(list: &[u8]) -> u8 {
    let mut multiplier = 1;
    let mut result: u8 = 0;

    for item in list.iter().rev() {
        result += item*multiplier;
        multiplier = multiplier*2;
    }
    result
}

pub fn octal_digit_from_binary_list_u16(list: &[u8]) -> u16 {
    let mut multiplier = 1;
    let mut result: u16 = 0;

    for item in list.iter().rev() {
        result += *item as u16 *multiplier;
        multiplier = multiplier*2;
    }
    result
}


#[test]
fn test_octal_digit() {
    assert_eq!(octal_digit_from_binary_list(&[0,0,0,1]), 1);
    assert_eq!(octal_digit_from_binary_list(&[1,0,0]), 4);
    assert_eq!(octal_digit_from_binary_list(&[1,1,1,1,1,1,1]), 127);
    assert_eq!(octal_digit_from_binary_list(&[1,1,1,1,1,1,0]), 126);
    assert_eq!(octal_digit_from_binary_list(&[0,1,1,1,1,1,0]), 126-64);

}

#[test]
fn test_octal_digit_u16() {
    assert_eq!(octal_digit_from_binary_list_u16(&[0,0,0,1]), 1);
    assert_eq!(octal_digit_from_binary_list_u16(&[1,0,0]), 4);
    assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1]), 127);
    assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,0]), 126);
    assert_eq!(octal_digit_from_binary_list_u16(&[0,1,1,1,1,1,0]), 126-64);

    assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1]), 255);
    assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1,1]), 511);
    assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1,1,1]), 1023);
    assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1,1,1,1]), 2047);

}