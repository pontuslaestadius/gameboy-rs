#![feature(slice_patterns)]
#![feature(inclusive_range_syntax)]


pub mod register;
pub mod instructions;



/// Decoding reading material:
/// Theory: http://www.z80.info/decoding.htm
/// op code: http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode
use std::io::prelude::*;
use std::fs::File;
use std::char;
use std::io::Error;
use std::fmt;
use instructions::Opcode;

use std::fmt::Debug;

use std::io;



use std::fs::OpenOptions;

struct Session {
    rom: Rom,
    registers: register::Registers,
}

impl Session {

    /// Steps through to the next instruction to be read and returns the byte.
    pub fn step(&mut self) -> &u8 {
        let old_pc = self.registers.get_pc();
        self.registers.set_pc(old_pc +1);
        self.rom.get(old_pc)
    }
}

struct Rom {
    content: Vec<u8>,
}

impl Rom {
    pub fn get(&self, index: u16) -> &u8 {
        self.content.get(index as usize).unwrap()
    }
}


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
        let ft = |x| {
            if x {
                1
            } else {
                0
            }
        };
        write!(f, "SmartBinary: [{},{},{},{},{},{},{},{}]",
            ft(self.zer),
            ft(self.one),
            ft(self.two),
            ft(self.thr),
            ft(self.fou),
            ft(self.fiv),
            ft(self.six),
            ft(self.sev)
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

pub fn rom_exec(file: &mut File) -> Result<(), io::Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let rom_size = file.read_to_end(&mut buffer)?;

    let rom = Rom {
        content: buffer
    };

    // Creates the registers.
    let registers = register::Registers::new();

    let mut session = Session {
        rom,
        registers,
    };

    println!("rom size: {}", rom_size);

    let mut bytes = 0;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("log.txt")?;

    while bytes <2000 {
        bytes  += 1;

        let step = session.step();

        let sb = SmartBinary::new(step.clone());

        let opcode = instructions::table::unprefixed_opcodes(sb);
        let formatted_opcode = format!("{:?}", opcode);

        match opcode {
            Opcode::INVALID(_) => {
                file.write_all(formatted_opcode.as_bytes());
                file.write(b"\n");
            }
            _ => println!("{}", formatted_opcode)
        }

        //println!("HEX: {:#X} U8: {} BINARY: {:b}", step, step, step);

    };

    Ok(())
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

#[test]
fn test_octal_digit() {
    assert_eq!(octal_digit_from_binary_list(&[0,0,0,1]), 1);
    assert_eq!(octal_digit_from_binary_list(&[1,0,0]), 4);
    assert_eq!(octal_digit_from_binary_list(&[1,1,1,1,1,1,1]), 127);
    assert_eq!(octal_digit_from_binary_list(&[1,1,1,1,1,1,0]), 126);
    assert_eq!(octal_digit_from_binary_list(&[0,1,1,1,1,1,0]), 126-64);

}