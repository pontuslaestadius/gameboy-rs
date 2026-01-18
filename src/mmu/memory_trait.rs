pub trait Memory {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn increment_cycles(&mut self, value: u64);

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
}
