use crate::constants::*;

const APU_READ_MASKS: [u8; 48] = [
    0x80, 0x3F, 0x00, 0xFF, 0xBF, // NR10 - NR14
    0xFF, 0x3F, 0x00, 0xFF, 0xBF, // NR20 - NR24 (NR20 is unused)
    0x7F, 0xFF, 0x9F, 0xFF, 0xBF, // NR30 - NR34
    0xFF, 0xFF, 0x00, 0x00, 0xBF, // NR40 - NR44 (NR40 is unused)
    0x00, 0x00, 0x70, // NR50 - NR52
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // Unused
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Wave RAM (No mask)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub struct Apu {
    enabled: bool,
    registers: [u8; 0x30], // 0xFF10 to 0xFF3F
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    pub fn new() -> Self {
        Self {
            enabled: false,
            registers: [0; 0x30],
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        let offset = (addr - 0xFF10) as usize;

        // When APU is disabled, most registers read as 0 (before masking)
        // Wave RAM (0xFF30-0xFF3F) is usually still accessible on DMG
        let val = if !self.enabled && addr < ADDR_APU_WAVE_START {
            0x00
        } else {
            self.registers[offset]
        };

        // Apply the hardware mask so unused bits return '1'
        val | APU_READ_MASKS[offset]
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        let offset = (addr - 0xFF10) as usize;

        if addr == ADDR_APU_NR52 {
            let was_enabled = self.enabled;
            self.enabled = (val & 0x80) != 0;

            if was_enabled && !self.enabled {
                // Powering down: Clear all registers 0xFF10-0xFF25
                for i in 0..0x16 {
                    self.registers[i] = 0;
                }
            }
            // Note: NR52 only has bit 7 as writable
            self.registers[offset] = val & 0x80;
            return;
        }

        // If the APU is off, writes to 0xFF10-0xFF25 are ignored
        if !self.enabled && addr < ADDR_APU_WAVE_START {
            return;
        }

        self.registers[offset] = val;
    }
}
