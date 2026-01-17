use crate::cartridge::*;

pub struct Cartridge {
    pub mbc: Box<dyn Mbc>, // Use a Box to hold any struct that implements Mbc
}

impl Cartridge {
    pub fn new(content: Vec<u8>) -> Self {
        // Logic to check byte 0x0147 in the ROM header
        // to see which MBC chip the game uses.
        let mbc_type = content[0x0147];

        let mbc: Box<dyn Mbc> = match mbc_type {
            0x00 => Box::new(RomOnly::new(content)),
            0x01..=0x03 => todo!("Implement MBC1"),
            _ => panic!("Unsupported MBC type: 0x{:02X}", mbc_type),
        };

        Cartridge { mbc }
    }
}
