use crate::constants::*;

// --- Sub-Components ---

#[derive(Default)]
pub struct LengthCounter {
    pub counter: u16,          // Internal counter
    pub enabled: bool,         // "Consecutive selection" (Bit 6 of NRx4)
    pub channel_enabled: bool, // Is the channel currently outputting?
}

impl LengthCounter {
    pub fn tick(&mut self) {
        if self.enabled && self.counter > 0 {
            self.counter -= 1;
            if self.counter == 0 {
                self.channel_enabled = false;
            }
        }
    }

    pub fn reload(&mut self, val: u16) {
        if self.counter == 0 {
            self.counter = val;
        }
    }
}

#[derive(Default)]
pub struct VolumeEnvelope {
    pub initial_volume: u8, // NRx2 Bits 4-7
    pub direction: bool,    // NRx2 Bit 3 (1=Up, 0=Down)
    pub period: u8,         // NRx2 Bits 0-2
    pub timer: u8,          // Internal timer
    pub current_volume: u8, // Actual output volume
}

impl VolumeEnvelope {
    pub fn tick(&mut self) {
        if self.period == 0 {
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
        }

        if self.timer == 0 {
            self.timer = self.period;
            if self.direction && self.current_volume < 15 {
                self.current_volume += 1;
            } else if !self.direction && self.current_volume > 0 {
                self.current_volume -= 1;
            }
        }
    }

    pub fn trigger(&mut self) {
        self.timer = if self.period > 0 { self.period } else { 8 };
        self.current_volume = self.initial_volume;
    }
}

#[derive(Default)]
pub struct FrequencySweep {
    pub period: u8,   // NR10 Bits 4-6
    pub negate: bool, // NR10 Bit 3
    pub shift: u8,    // NR10 Bits 0-2
    pub timer: u8,
    pub shadow_freq: u16,
    pub enabled: bool,
}

impl FrequencySweep {
    pub fn tick(&mut self, channel_freq: &mut u16, channel_active: &mut bool) {
        if self.timer > 0 {
            self.timer -= 1;
        }

        if self.timer == 0 {
            self.timer = if self.period > 0 { self.period } else { 8 };

            if self.enabled && self.period > 0 {
                let new_freq = self.calculate_freq(*channel_freq);

                if new_freq <= 2047 && self.shift > 0 {
                    *channel_freq = new_freq;
                    self.shadow_freq = new_freq;

                    // Overflow check immediately after write
                    if self.calculate_freq(new_freq) > 2047 {
                        *channel_active = false;
                    }
                } else if new_freq > 2047 {
                    *channel_active = false;
                }
            }
        }
    }

    pub fn calculate_freq(&self, freq: u16) -> u16 {
        let offset = freq >> self.shift;
        if self.negate {
            freq.wrapping_sub(offset)
        } else {
            freq.wrapping_add(offset)
        }
    }

    pub fn trigger(&mut self, freq: u16, channel_active: &mut bool) {
        self.shadow_freq = freq;
        self.timer = if self.period > 0 { self.period } else { 8 };
        self.enabled = self.period > 0 || self.shift > 0;

        // Test 06 requirement: If shift > 0, calculate immediately
        if self.shift > 0 {
            if self.calculate_freq(self.shadow_freq) > 2047 {
                *channel_active = false;
            }
        }
    }
}

// --- Specialized Channels ---

#[derive(Default)]
pub struct Channel1 {
    pub sweep: FrequencySweep,
    pub length: LengthCounter,
    pub envelope: VolumeEnvelope,
    pub duty: u8,
    pub frequency: u16,
}

#[derive(Default)]
pub struct Channel2 {
    pub length: LengthCounter,
    pub envelope: VolumeEnvelope,
    pub duty: u8,
    pub frequency: u16,
}

#[derive(Default)]
pub struct Channel3 {
    pub enabled: bool, // NR30 Bit 7
    pub length: LengthCounter,
    pub output_level: u8, // NR32 Bits 5-6
    pub frequency: u16,
    pub wave_ram: [u8; 16],
    pub position_counter: u8, // Internal 0-31
}

#[derive(Default)]
pub struct Channel4 {
    pub length: LengthCounter,
    pub envelope: VolumeEnvelope,
    pub polynomial: u8, // NR43
    pub lfsr: u16,      // Linear Feedback Shift Register
}

// --- Main APU Module ---

pub struct Apu {
    pub enabled: bool, // Master Power (NR52 Bit 7)
    pub fs_timer: u32, // Frame Sequencer Timer
    pub fs_step: u8,   // Frame Sequencer Step (0-7)

    pub ch1: Channel1,
    pub ch2: Channel2,
    pub ch3: Channel3,
    pub ch4: Channel4,

    pub nr50: u8, // Vol / Vin
    pub nr51: u8, // Panning
}

impl Apu {
    pub fn new() -> Self {
        Self {
            enabled: false,
            fs_timer: 0,
            fs_step: 0,
            ch1: Channel1::default(),
            ch2: Channel2::default(),
            ch3: Channel3::default(),
            ch4: Channel4::default(),
            nr50: 0,
            nr51: 0,
        }
    }

    // -------------------------------------------------------------------------
    //  Core Interconnect Logic
    // -------------------------------------------------------------------------

    /// Called by Bus every T-Cycle (approx 4 MHz)
    pub fn tick(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        self.fs_timer += cycles;

        // 512 Hz Frame Sequencer (Approx every 8192 T-cycles)
        if self.fs_timer >= 8192 {
            self.fs_timer -= 8192;
            self.advance_frame_sequencer();
        }

        // TODO: Tick channel frequency timers here for audio generation
    }

    fn advance_frame_sequencer(&mut self) {
        match self.fs_step {
            0 => self.clock_lengths(),
            1 => {}
            2 => {
                self.clock_lengths();
                self.clock_sweep();
            }
            3 => {}
            4 => self.clock_lengths(),
            5 => {}
            6 => {
                self.clock_lengths();
                self.clock_sweep();
            }
            7 => self.clock_envelopes(),
            _ => unreachable!(),
        }
        self.fs_step = (self.fs_step + 1) % 8;
    }

    fn clock_lengths(&mut self) {
        self.ch1.length.tick();
        self.ch2.length.tick();
        self.ch3.length.tick();
        self.ch4.length.tick();
    }

    fn clock_envelopes(&mut self) {
        self.ch1.envelope.tick();
        self.ch2.envelope.tick();
        self.ch4.envelope.tick();
    }

    fn clock_sweep(&mut self) {
        self.ch1.sweep.tick(
            &mut self.ch1.frequency,
            &mut self.ch1.length.channel_enabled,
        );
    }

    // -------------------------------------------------------------------------
    //  Bus Read
    // -------------------------------------------------------------------------

    pub fn read_byte(&self, addr: u16) -> u8 {
        if !self.enabled && addr != ADDR_APU_NR52 {
            // Unmapped / Powered Down reads (except Wave RAM in some revs)
            if (ADDR_APU_WAVE_START..=ADDR_APU_WAVE_END).contains(&addr) {
                return 0xFF; // TODO: Exact behavior depends on DMG revision
            }
            // Return strict hardware masks when off
            return match addr {
                ADDR_APU_NR10 => 0x80,
                ADDR_APU_NR11 => 0x3F,
                ADDR_APU_NR12 => 0x00,
                ADDR_APU_NR14 => 0xBF,
                ADDR_APU_NR21 => 0x3F,
                ADDR_APU_NR22 => 0x00,
                ADDR_APU_NR24 => 0xBF,
                ADDR_APU_NR30 => 0x7F,
                ADDR_APU_NR32 => 0x9F,
                ADDR_APU_NR34 => 0xBF,
                ADDR_APU_NR42 => 0x00,
                ADDR_APU_NR43 => 0x00,
                ADDR_APU_NR44 => 0xBF,
                _ => 0xFF,
            };
        }

        match addr {
            // Ch 1
            ADDR_APU_NR10 => {
                0x80 | (self.ch1.sweep.period << 4)
                    | (if self.ch1.sweep.negate { 0x08 } else { 0 })
                    | self.ch1.sweep.shift
            }
            ADDR_APU_NR11 => 0x3F | (self.ch1.duty << 6),
            ADDR_APU_NR12 => {
                (self.ch1.envelope.initial_volume << 4)
                    | (if self.ch1.envelope.direction { 0x08 } else { 0 })
                    | self.ch1.envelope.period
            }
            ADDR_APU_NR13 => 0xFF,
            ADDR_APU_NR14 => 0xBF | (if self.ch1.length.enabled { 0x40 } else { 0 }),

            // Ch 2
            ADDR_APU_NR21 => 0x3F | (self.ch2.duty << 6),
            ADDR_APU_NR22 => {
                (self.ch2.envelope.initial_volume << 4)
                    | (if self.ch2.envelope.direction { 0x08 } else { 0 })
                    | self.ch2.envelope.period
            }
            ADDR_APU_NR23 => 0xFF,
            ADDR_APU_NR24 => 0xBF | (if self.ch2.length.enabled { 0x40 } else { 0 }),

            // Ch 3
            ADDR_APU_NR30 => 0x7F | (if self.ch3.enabled { 0x80 } else { 0 }),
            ADDR_APU_NR31 => 0xFF,
            ADDR_APU_NR32 => 0x9F | (self.ch3.output_level << 5),
            ADDR_APU_NR33 => 0xFF,
            ADDR_APU_NR34 => 0xBF | (if self.ch3.length.enabled { 0x40 } else { 0 }),

            // Ch 4
            ADDR_APU_NR41 => 0xFF,
            ADDR_APU_NR42 => {
                (self.ch4.envelope.initial_volume << 4)
                    | (if self.ch4.envelope.direction { 0x08 } else { 0 })
                    | self.ch4.envelope.period
            }
            ADDR_APU_NR43 => self.ch4.polynomial,
            ADDR_APU_NR44 => 0xBF | (if self.ch4.length.enabled { 0x40 } else { 0 }),

            // Control
            ADDR_APU_NR50 => self.nr50,
            ADDR_APU_NR51 => self.nr51,
            ADDR_APU_NR52 => {
                let mut status = if self.enabled { 0xF0 } else { 0x70 };
                if self.ch1.length.channel_enabled {
                    status |= 0x01;
                }
                if self.ch2.length.channel_enabled {
                    status |= 0x02;
                }
                if self.ch3.length.channel_enabled {
                    status |= 0x04;
                }
                if self.ch4.length.channel_enabled {
                    status |= 0x08;
                }
                status
            }

            ADDR_APU_WAVE_START..=ADDR_APU_WAVE_END => {
                if self.ch3.length.channel_enabled {
                    // If playing, DMG returns the byte at the current position counter
                    let pos = self.ch3.position_counter / 2;
                    self.ch3.wave_ram[pos as usize]
                } else {
                    self.ch3.wave_ram[(addr - ADDR_APU_WAVE_START) as usize]
                }
            }

            _ => 0xFF,
        }
    }

    // -------------------------------------------------------------------------
    //  Bus Write
    // -------------------------------------------------------------------------

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        // NR52 Power Check: If off, only NR52 is writable.
        if !self.enabled && addr != ADDR_APU_NR52 {
            // Exception: Wave RAM is writable on DMG even if off (usually)
            // Exception: Length counters (NRx1) are writable even if off
            if !((ADDR_APU_WAVE_START..=ADDR_APU_WAVE_END).contains(&addr))
                && addr != ADDR_APU_NR11
                && addr != ADDR_APU_NR21
                && addr != ADDR_APU_NR31
                && addr != ADDR_APU_NR41
            {
                return;
            }
        }

        match addr {
            // --- Ch 1 ---
            ADDR_APU_NR10 => {
                self.ch1.sweep.period = (val >> 4) & 0x07;
                self.ch1.sweep.negate = (val & 0x08) != 0;
                self.ch1.sweep.shift = val & 0x07;
            }
            ADDR_APU_NR11 => {
                self.ch1.duty = (val >> 6) & 0x03;
                self.ch1.length.counter = 64 - (val & 0x3F) as u16;
            }
            ADDR_APU_NR12 => {
                self.ch1.envelope.initial_volume = val >> 4;
                self.ch1.envelope.direction = (val & 0x08) != 0;
                self.ch1.envelope.period = val & 0x07;
            }
            ADDR_APU_NR13 => {
                self.ch1.frequency = (self.ch1.frequency & 0xFF00) | val as u16;
            }
            ADDR_APU_NR14 => {
                self.ch1.frequency = (self.ch1.frequency & 0x00FF) | ((val as u16 & 0x07) << 8);
                self.ch1.length.enabled = (val & 0x40) != 0;
                if (val & 0x80) != 0 {
                    // Trigger
                    self.ch1.length.reload(64);
                    self.ch1.length.channel_enabled = true;
                    self.ch1.envelope.trigger();
                    self.ch1
                        .sweep
                        .trigger(self.ch1.frequency, &mut self.ch1.length.channel_enabled);
                }
            }

            // --- Ch 2 ---
            ADDR_APU_NR21 => {
                self.ch2.duty = (val >> 6) & 0x03;
                self.ch2.length.counter = 64 - (val & 0x3F) as u16;
            }
            ADDR_APU_NR22 => {
                self.ch2.envelope.initial_volume = val >> 4;
                self.ch2.envelope.direction = (val & 0x08) != 0;
                self.ch2.envelope.period = val & 0x07;
            }
            ADDR_APU_NR23 => {
                self.ch2.frequency = (self.ch2.frequency & 0xFF00) | val as u16;
            }
            ADDR_APU_NR24 => {
                self.ch2.frequency = (self.ch2.frequency & 0x00FF) | ((val as u16 & 0x07) << 8);
                self.ch2.length.enabled = (val & 0x40) != 0;
                if (val & 0x80) != 0 {
                    self.ch2.length.reload(64);
                    self.ch2.length.channel_enabled = true;
                    self.ch2.envelope.trigger();
                }
            }

            // --- Ch 3 ---
            ADDR_APU_NR30 => {
                self.ch3.enabled = (val & 0x80) != 0;
                if !self.ch3.enabled {
                    self.ch3.length.channel_enabled = false;
                }
            }
            ADDR_APU_NR31 => {
                self.ch3.length.counter = 256 - val as u16;
            }
            ADDR_APU_NR32 => {
                self.ch3.output_level = (val >> 5) & 0x03;
            }
            ADDR_APU_NR33 => {
                self.ch3.frequency = (self.ch3.frequency & 0xFF00) | val as u16;
            }
            ADDR_APU_NR34 => {
                self.ch3.frequency = (self.ch3.frequency & 0x00FF) | ((val as u16 & 0x07) << 8);
                self.ch3.length.enabled = (val & 0x40) != 0;
                if (val & 0x80) != 0 {
                    self.ch3.length.reload(256);
                    self.ch3.length.channel_enabled = true;
                    self.ch3.position_counter = 0;
                }
            }

            // --- Ch 4 ---
            ADDR_APU_NR41 => {
                self.ch4.length.counter = 64 - (val & 0x3F) as u16;
            }
            ADDR_APU_NR42 => {
                self.ch4.envelope.initial_volume = val >> 4;
                self.ch4.envelope.direction = (val & 0x08) != 0;
                self.ch4.envelope.period = val & 0x07;
            }
            ADDR_APU_NR43 => {
                self.ch4.polynomial = val;
            }
            ADDR_APU_NR44 => {
                self.ch4.length.enabled = (val & 0x40) != 0;
                if (val & 0x80) != 0 {
                    self.ch4.length.reload(64);
                    self.ch4.length.channel_enabled = true;
                    self.ch4.envelope.trigger();
                    self.ch4.lfsr = 0x7FFF; // Reset LFSR
                }
            }

            // --- Control ---
            ADDR_APU_NR50 => self.nr50 = val,
            ADDR_APU_NR51 => self.nr51 = val,
            ADDR_APU_NR52 => {
                // Only Bit 7 is writable.
                let next_enabled = (val & 0x80) != 0;
                if self.enabled && !next_enabled {
                    // Powering OFF: clear registers
                    self.clear_registers();
                } else if !self.enabled && next_enabled {
                    // Powering ON: reset sequencer
                    self.fs_step = 0;
                    self.fs_timer = 0;
                    // Note: Wave RAM is NOT cleared on power on
                }
                self.enabled = next_enabled;
            }

            ADDR_APU_WAVE_START..=ADDR_APU_WAVE_END => {
                self.ch3.wave_ram[(addr - ADDR_APU_WAVE_START) as usize] = val;
            }

            _ => {}
        }
    }

    fn clear_registers(&mut self) {
        self.nr50 = 0;
        self.nr51 = 0;
        // Clear all channels EXCEPT Wave RAM
        self.ch1 = Channel1::default();
        self.ch2 = Channel2::default();
        // Manual reset for Ch3 to preserve wave_ram
        self.ch3.enabled = false;
        self.ch3.length = LengthCounter::default();
        self.ch3.output_level = 0;
        self.ch3.frequency = 0;
        self.ch4 = Channel4::default();
    }
}
