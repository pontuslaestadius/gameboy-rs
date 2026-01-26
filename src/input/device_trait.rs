pub trait InputDevice {
    fn tick(&mut self, cycles: u8);
    // selection: bit 4 = buttons, bit 5 = directions
    fn read(&self, selection: u8) -> u8;
}
