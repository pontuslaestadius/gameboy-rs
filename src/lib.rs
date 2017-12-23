
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


struct Rom {
    content: Vec<u8>,
}

impl Rom {
    pub fn get(&self, index: usize) -> &u8 {
        self.content.get(index).unwrap()
    }
}

pub fn rom_exec(mut file: &mut File) {
    let mut buffer: Vec<u8> = Vec::new();
    let rom_size = file.read_to_end(&mut buffer).unwrap();

    let rom = Rom {
        content: buffer
    };

    println!("rom size: {}", rom_size);

    let mut bytes = 0;

    while bytes <100 {
        bytes  += 1;



        println!("{:#X}", rom.get(bytes));

    }
}

pub fn read_char(file: &mut File) -> Result<char, Error> {
    let mut buffer: [u8; 1] = [0];
    file.read(&mut buffer)?;

    Ok(
        char::from_u32(buffer[0] as u32).unwrap()
    )
}


pub fn read_compare(mut file: &mut File, chars: &[char]) -> bool {

    for c in chars.iter() {
        if c != &read_char(&mut file).unwrap() {
            return false;
        }
    }
    true
}