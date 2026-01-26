use super::InputDevice;

#[derive(Default)]
pub struct RotaryInput {
    current_mask: u8, // The current bitmask of pressed buttons
    timer: u32,
    state_index: usize,
}

impl RotaryInput {
    pub fn new() -> Self {
        Self {
            current_mask: 0xFF, // All buttons released (1 = released)
            timer: 0,
            state_index: 0,
        }
    }
}

impl InputDevice for RotaryInput {
    fn read(&self, _selection: u8) -> u8 {
        self.current_mask
    }

    fn tick(&mut self, cycles: u8) {
        // Increment timer by CPU cycles
        self.timer = self.timer.wrapping_add(cycles as u32);

        // Every ~0.5 seconds (2,000,000 cycles at 4MHz), change the input
        if self.timer > 2_000_000 {
            self.timer = 0;
            self.state_index = (self.state_index + 1) % 6; // Cycle through 6 states

            self.current_mask = match self.state_index {
                0 => !0x08, // Start pressed (Bit 3)
                1 => 0xFF,  // All released (Gap for transition)
                2 => !0x01, // A pressed (Bit 0)
                3 => 0xFF,  // All released
                4 => !0x40, // Up pressed (Bit 6 - technically for direction line)
                5 => 0xFF,  // All released
                _ => 0xFF,
            };
        }
    }
}
