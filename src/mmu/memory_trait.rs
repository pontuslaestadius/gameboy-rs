use log::trace;

use crate::constants::{ADDR_TIMER_TIMA, IE_ADDR, IF_ADDR};

pub trait Memory {
    /// Read directly from the memory without allowing the bus to route
    /// it to any subcomponent, or inject any trace logging.
    fn read_byte_raw(&self, addr: u16) -> u8;
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, val: u8);
    fn force_write_byte(&mut self, addr: u16, val: u8);
    fn tick_components(&mut self, cycles: u8) -> bool;
    fn write_div(&mut self);

    // Helper for 16-bit reads (Little Endian)
    fn read_u16(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    // Helper for 16-bit writes (Little Endian)
    fn write_u16(&mut self, addr: u16, val: u16) {
        self.write_byte(addr, (val & 0xFF) as u8);
        self.write_byte(addr.wrapping_add(1), (val >> 8) as u8);
    }

    fn force_write_bytes(&mut self, start_addr: u16, bytes: &[u8]) {
        for (i, &byte) in bytes.iter().enumerate() {
            self.force_write_byte(start_addr + i as u16, byte);
        }
    }

    fn read_ie(&self) -> u8 {
        self.read_byte(IE_ADDR)
    }

    fn read_if(&self) -> u8 {
        self.read_byte(IF_ADDR)
    }

    fn read_tima(&self) -> u8 {
        self.read_byte(ADDR_TIMER_TIMA)
    }

    fn write_ie(&mut self, value: u8) {
        self.write_byte(IE_ADDR, value);
    }

    fn write_if(&mut self, value: u8) {
        self.write_byte(IF_ADDR, value);
    }

    fn write_tima(&mut self, value: u8) {
        self.write_byte(ADDR_TIMER_TIMA, value);
    }

    fn pending_interrupt(&self) -> bool {
        let if_val = self.read_byte_raw(IF_ADDR);
        let ie_val = self.read_byte_raw(IE_ADDR);
        let val = ((if_val & ie_val) & 0x1F) != 0;
        // Needs to be 6 char width, that's why the awkward spacing.
        trace!("pending interrupt -> {}", val);
        val
    }
}
