use crate::constants::{IE_ADDR, IF_ADDR};

pub trait Memory {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn increment_cycles(&mut self, value: u64);
    fn tick_components(&mut self, cycles: u8);
    fn write_div(&mut self);

    // Helper for 16-bit reads (Little Endian)
    fn read_u16(&self, addr: u16) -> u16 {
        let low = self.read(addr) as u16;
        let high = self.read(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    // Helper for 16-bit writes (Little Endian)
    fn write_u16(&mut self, addr: u16, val: u16) {
        self.write(addr, (val & 0xFF) as u8);
        self.write(addr.wrapping_add(1), (val >> 8) as u8);
    }

    fn read_ie(&self) -> u8 {
        self.read(IE_ADDR)
    }

    fn read_if(&self) -> u8 {
        self.read(IF_ADDR)
    }

    fn read_byte(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write_byte(&mut self, addr: u16, val: u8) {
        self.write(addr, val);
    }

    fn write_ie(&mut self, value: u8) {
        self.write(IE_ADDR, value);
    }

    fn write_if(&mut self, value: u8) {
        self.write(IF_ADDR, value);
    }

    fn pending_interrupt(&self) -> bool {
        ((self.read(IF_ADDR) & self.read(IE_ADDR)) & 0x1F) != 0
    }
}
