
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
struct SmartBinary {
    zer: bool,
    one: bool,
    two: bool,
    thr: bool,
    fou: bool,
    fiv: bool,
    six: bool,
    sev: bool,
}

impl SmartBinary {
    pub fn new(byte: u8) -> SmartBinary {
        // TODO
        let formatted = format!("{:b}", byte);

        let mut formatted_chars = formatted.chars();

        let cnvt = |x| x == '1';

        SmartBinary {
            zer: cnvt(formatted_chars.nth(0).unwrap()),
            one: cnvt(formatted_chars.nth(1).unwrap()),
            two: cnvt(formatted_chars.nth(2).unwrap()),
            thr: cnvt(formatted_chars.nth(3).unwrap()),
            fou: cnvt(formatted_chars.nth(4).unwrap()),
            fiv: cnvt(formatted_chars.nth(5).unwrap()),
            six: cnvt(formatted_chars.nth(6).unwrap()),
            sev: cnvt(formatted_chars.nth(7).unwrap()),
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

}

pub fn rom_exec(mut file: &mut File) {
    let mut buffer: Vec<u8> = Vec::new();
    let rom_size = file.read_to_end(&mut buffer).unwrap();

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

    while bytes <200 {
        bytes  += 1;

        let step = session.step();
        println!("HEX: {:#X} U8: {} BINARY: {:b}", step, step, step);

    }
}

pub fn prefix_table(byte: u8) {

}

pub fn opcode_table(byte: u8) {

}