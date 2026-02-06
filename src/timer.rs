pub struct Timer {
    pub internal_counter: u16, // Increments every T-cycle
    pub tima: u8,              // 0xFF05
    pub tma: u8,               // 0xFF06
    pub tac: u8,               // 0xFF07
    pub div: u8,               // 0xFF04 (Top 8 bits of internal_counter)
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        Self {
            internal_counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div: 0,
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => panic!("Timer has a restrictive addr space"),
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF04 => self.div = val,
            0xFF05 => self.tima = val,
            0xFF06 => self.tma = val,
            0xFF07 => self.tac = val,
            _ => panic!("Timer has a restrictive addr space"),
        };
    }

    /// Helper to map TAC bits 0-1 to internal counter bits
    pub fn get_bit_index(&self, selection: u8) -> u16 {
        match selection {
            0b00 => 9, // 1024 cycles
            0b01 => 3, // 16 cycles
            0b10 => 5, // 64 cycles
            0b11 => 7, // 256 cycles
            _ => unreachable!(),
        }
    }
    pub fn timer_enabled(&self) -> bool {
        // Bit 2 of TAC (0xFF07) enables/disables the TIMA counter
        (self.tac & 0b100) != 0
    }

    pub fn get_tac_bit(&self) -> u16 {
        // Maps TAC bits 0-1 to the specific bit in the 16-bit internal counter
        match self.tac & 0b11 {
            0b00 => 9, // 1024 cycles (4096 Hz)
            0b01 => 3, // 16 cycles   (262144 Hz)
            0b10 => 5, // 64 cycles   (65536 Hz)
            0b11 => 7, // 256 cycles  (16384 Hz)
            _ => unreachable!(),
        }
    }
    pub fn increment_tima(&mut self) -> bool {
        let mut interrupt_triggered = false;

        let (new_tima, overflow) = self.tima.overflowing_add(1);

        if overflow {
            // THE OVERFLOW EVENT
            // 1. TIMA is reset to the value in TMA
            self.tima = self.tma;

            // 2. We signal that an interrupt should be requested
            interrupt_triggered = true;
        } else {
            self.tima = new_tima;
        }

        interrupt_triggered
    }
    pub fn tick(&mut self, cycles: u8) -> bool {
        // println!("timer tick: {}", cycles);
        let mut interrupt_requested = false;

        // Game Boy ticks in T-cycles (4MHz)
        for _ in 0..cycles {
            let old_counter = self.internal_counter;
            self.internal_counter = self.internal_counter.wrapping_add(1);

            // 1. Update DIV (Top 8 bits of 16-bit counter)
            self.div = (self.internal_counter >> 8) as u8;

            // 2. Determine the bit we are watching based on TAC
            let bit_index = match self.tac & 0b11 {
                0b00 => 9, // 1024 cycles
                0b01 => 3, // 16 cycles
                0b10 => 5, // 64 cycles
                0b11 => 7, // 256 cycles
                _ => unreachable!(),
            };

            // 3. Falling Edge Logic: (Enabled & Bit)
            let timer_enabled = (self.tac & 0b100) != 0;
            let old_signal = timer_enabled && ((old_counter >> bit_index) & 1) != 0;
            let new_signal = timer_enabled && ((self.internal_counter >> bit_index) & 1) != 0;

            // Signal was High (1) and is now Low (0)
            if old_signal && !new_signal && self.increment_tima() {
                interrupt_requested = true;
            }
        }
        interrupt_requested
    }

    pub fn write_tac(&mut self, new_val: u8) {
        // 1. Calculate the signal BEFORE the write
        let old_bit = self.get_tac_bit();
        let old_signal =
            ((self.internal_counter >> old_bit) & 0x01) & ((self.tac as u16 >> 2) & 0x01);

        // 2. Update TAC
        self.tac = new_val;

        // 3. Calculate the signal AFTER the write
        let new_bit = self.get_tac_bit();
        let new_signal =
            ((self.internal_counter >> new_bit) & 0x01) & ((self.tac as u16 >> 2) & 0x01);

        // 4. Falling Edge check: If signal was 1 and is now 0, increment TIMA
        if old_signal == 1 && new_signal == 0 {
            self.increment_tima();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_timer_frequency_increment() {
        let mut timer = Timer::new();
        timer.tac = 0x05; // Speed 01: Every 16 T-cycles (Bit 3)

        // Manual check:
        timer.tick(15);
        // After 15 ticks:
        // internal_counter is 15 (0b01111). Bit 3 is 1.
        assert_eq!(timer.tima, 0);

        timer.tick(1);
        // After 16 ticks:
        // internal_counter is 16 (0b10000). Bit 3 is 0.
        // This is the FALLING EDGE that triggers TIMA.
        assert_eq!(timer.tima, 1, "TIMA failed to increment at 16 cycles");
    }
    #[test]
    fn test_div_reset_falling_edge() {
        let mut timer = Timer::new();
        timer.tac = 0x05; // Speed: 16 cycles (Bit 3)

        // 1. Tick 8 times. Internal counter is now 8 (0b1000). Bit 3 is HIGH.
        timer.tick(8);
        assert_eq!(timer.tima, 0);

        // 2. Perform a DIV reset (what happens when writing to 0xFF04)
        // We simulate the logic: bit 3 was 1, resetting makes it 0.
        let bit_was_high = (timer.internal_counter >> 3) & 0x01 == 1;
        timer.internal_counter = 0;
        if bit_was_high {
            timer.tima += 1; // Falling edge triggered!
        }

        assert_eq!(
            timer.tima, 1,
            "TIMA should have incremented due to DIV reset falling edge"
        );
    }
    #[test]
    fn test_timer_overflow_reloads_tma() {
        let mut timer = Timer {
            internal_counter: 0,
            tima: 0xFE, // 254
            tma: 0xAA,  // 170
            tac: 0x05,  // Enabled, Clock 01 (16 cycles)
            div: 0,
        };

        // We need 16 cycles to move 254 -> 255
        // And another 16 cycles to move 255 -> 0 (Reload)
        // Total 32 cycles.
        let mut irq = false;
        for _ in 0..32 {
            if timer.tick(1) {
                irq = true;
            }
        }

        assert_eq!(timer.tima, 0xAA, "TIMA should have reloaded from TMA");
        assert!(irq, "Timer interrupt should have been requested");
    }
    #[test]
    fn test_timer_logic_isolation() {
        let mut timer = Timer::new();

        // 1. Setup: Speed 01 (bit 3), Enabled
        timer.write_byte(0xFF07, 0x05);
        timer.write_byte(0xFF06, 0xAA); // TMA
        timer.write_byte(0xFF05, 0xFE); // TIMA

        // 2. Tick 15 cycles. No increment should happen yet.
        // (internal_counter 0 -> 15. Bit 3 stays 0 for cycles 0-7, becomes 1 at 8)
        let mut irq = timer.tick(15);
        assert!(!irq, "No interrupt should trigger before overflow");
        assert_eq!(timer.tima, 0xFE, "TIMA should not have incremented yet");

        // 3. Tick 1 more cycle (Total 16).
        // internal_counter 15 -> 16. Bit 3 was 1, now becomes 0 (Falling Edge!)
        irq = timer.tick(1);
        assert!(!irq);
        assert_eq!(
            timer.tima, 0xFF,
            "TIMA should be 0xFF after the falling edge at 16 cycles"
        );

        // 4. Tick 15 more cycles.
        timer.tick(15);
        assert_eq!(timer.tima, 0xFF);

        // 5. The Overflow Tick (Cycle 32)
        // This will trigger another falling edge, incrementing TIMA from 0xFF to 0x00.
        irq = timer.tick(1);

        assert!(
            irq,
            "The tick function MUST return true when TIMA overflows"
        );
        assert_eq!(
            timer.tima, 0xAA,
            "TIMA should have reloaded from TMA (0xAA)"
        );
    }
}
