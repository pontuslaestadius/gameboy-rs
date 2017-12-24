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

    let mut invalid = 0;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("log.txt")?;

    while bytes <rom_size {
        bytes  += 1;

        let opcode: Opcode = session.unprefixed_opcodes();
        let formatted_opcode: String = format!("{:?}", opcode);

        match opcode {
            Opcode::INVALID(_) => {
                file.write_all(formatted_opcode.as_bytes());
                file.write(b"\n");
                invalid += 1;
            }

            _ => println!("{}", formatted_opcode)
        }


        //println!("HEX: {:#X} U8: {} BINARY: {:b}", step, step, step);

    };

    println!("valid: {}", rom_size-invalid);
    println!("invalid: {}", invalid);
    println!("valid/invalid: {}%", (rom_size-invalid) as f64/invalid as f64);


    Ok(())
}
