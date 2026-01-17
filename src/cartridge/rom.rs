use crate::cartridge::mbc_trait::Mbc;

/// Holds the content of the rom, As to load it in to memory.
pub struct RomOnly {
    data: Vec<u8>,
}

impl RomOnly {
    pub fn new(content: Vec<u8>) -> Self {
        Self { data: content }
    }
}

impl Mbc for RomOnly {
    fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    fn write(&mut self, _addr: u16, _val: u8) {
        // You can't write to ROM!
        // Maybe panic?
    }
}
