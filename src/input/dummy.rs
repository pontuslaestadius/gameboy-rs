use super::InputDevice;

#[derive(Default)]
pub struct DummyInput;

impl InputDevice for DummyInput {
    fn tick(&mut self, _cycles: u8) {}

    fn read(&self, selection: u8) -> u8 {
        // Return 0xCF (nothing pressed) + selection bits
        // This prevents the game from reading random '0's and crashing
        0xCF | (selection & 0x30)
    }
}
