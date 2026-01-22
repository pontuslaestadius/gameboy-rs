/*
Source: https://gbdev.io/pandocs/Memory_Map.html

The Game Boy has a 16-bit address bus, which is used to address ROM, RAM, and I/O.

Start	End	Description	Notes
0000	3FFF	16 KiB ROM bank 00	From cartridge, usually a fixed bank
4000	7FFF	16 KiB ROM Bank 01–NN	From cartridge, switchable bank via mapper (if any)
8000	9FFF	8 KiB Video RAM (VRAM)	In CGB mode, switchable bank 0/1
A000	BFFF	8 KiB External RAM	From cartridge, switchable bank if any
C000	CFFF	4 KiB Work RAM (WRAM)
D000	DFFF	4 KiB Work RAM (WRAM)	In CGB mode, switchable bank 1–7
E000	FDFF	Echo RAM (mirror of C000–DDFF)	Nintendo says use of this area is prohibited.
FE00	FE9F	Object attribute memory (OAM)
FEA0	FEFF	Not Usable	Nintendo says use of this area is prohibited.
FF00	FF7F	I/O Registers
FF80	FFFE	High RAM (HRAM)
FFFF	FFFF	Interrupt Enable register (IE)

We make an abstraction, and don't store it 1-1.
*/

use log::{debug, info};

use crate::{
    mmu::memory_trait::Memory,
    ppu::{DummyPpu, Ppu},
    timer::Timer,
};

/// 64 Kb - The standard Game Boy address space
const MEMORY_SIZE: usize = 1024 * 64;

pub struct Bus {
    timer: Timer,
    // Must use a Vec since an Array would use the stack, and crash the application.
    // Using the heap is required.
    pub rom_size: usize,
    // This puts exactly 64KB on the HEAP, not the STACK
    pub data: Box<[u8; MEMORY_SIZE]>,
    // total_cycles: u64,
    pub ppu: Box<dyn Ppu>,

    pub joypad_sel: u8,
}

impl Bus {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let rom_size = rom_data.len();
        // Create a zeroed array on the heap
        let mut buffer = Box::new([0u8; MEMORY_SIZE]);

        // Copy ROM data into the beginning
        let copy_len = std::cmp::min(rom_size, MEMORY_SIZE);
        buffer[..copy_len].copy_from_slice(&rom_data[..copy_len]);

        debug!(
            "Creating Bus, memory_size: {}, rom_size: {}, copy_len: {}",
            MEMORY_SIZE,
            rom_data.len(),
            copy_len
        );

        Bus {
            timer: Timer::new(),
            rom_size,
            data: buffer,
            ppu: Box::new(DummyPpu::new()),
            joypad_sel: 0xFF,
        }
    }
}

impl Bus {}

impl Memory for Bus {
    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // ROM: Fixed and Switchable Banks
            // 0x0000..=0x7FFF => self.rom[addr as usize],

            // VRAM: Owned by PPU
            0x8000..=0x9FFF => self.ppu.read_byte(addr),

            // External RAM (Cartridge)
            // 0xA000..=0xBFFF => self.ext_ram[(addr - 0xA000) as usize],

            // Work RAM (WRAM)
            0xC000..=0xDFFF => self.data[addr as usize],

            // Echo RAM: Subtract 0x2000 to redirect to WRAM
            0xE000..=0xFDFF => self.data[(addr - 0x2000) as usize],

            // OAM: Owned by PPU
            0xFE00..=0xFE9F => self.ppu.read_byte(addr),

            // Forbidden Zone: Usually returns 0 or 0xFF
            0xFEA0..=0xFEFF => 0x00,

            // I/O Registers
            // 0xFF00 => 0xCF, // Joypad placeholder (all buttons up)
            // 0xFF00 => {
            // Basic logic: if the CPU is looking for Buttons (Bit 5 is 0)
            // return 0xF7 (1111 0111) to signal 'Start' is pressed.
            // Otherwise, return 0xFF (nothing pressed).
            // if (self.joypad_reg & 0x20) == 0 {
            //     0xF7
            // } else {
            //     0xFF
            // }
            // }
            0xFF00 => {
                match self.joypad_sel {
                    0x10 => 0xF7, // Selecting buttons: return Start pressed
                    0x20 => 0xFF, // Selecting directions: nothing pressed
                    _ => 0xFF,
                }
            }

            0xFF04..=0xFF07 => self.timer.read_byte(addr),
            // Handled by CPU.
            // 0xFF0F => self.interrupt_flags,

            // PPU Registers
            0xFF40..=0xFF4B => self.ppu.read_byte(addr),

            // High RAM (HRAM)
            // 0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],

            // Interrupt Enable, handled by CPU.
            // 0xFFFF => self.interrupt_enable,

            // Default: Return data from the main block or 0xFF
            _ => self.data[addr as usize],
        }

        // // 0x0000..=0x7FFF => self.rom.read(addr),
        // 0x8000..=0x9FFF => self.ppu.read_byte(addr), // VRAM
        // // 0xC000..=0xDFFF => self.wram[addr - 0xC000],
        // 0xFE00..=0xFE9F => self.ppu.read_byte(addr), // OAM
        // 0xFF40..=0xFF4B => self.ppu.read_byte(addr), // PPU Registers
        // // If we are in the middle of a CPU test, just return 0x90
        // // to let the CPU pass the 'Wait for V-Blank' loop.
        // //         // Return a rotating value to satisfy "Wait for LY == X" loops
        // //         // This is a common hack for CPU-only testing
        // // return ((self.total_cycles) / 456 % 154) as u8;
        // 0xFF44 => 0x90,
        // _ => self.data[addr as usize],
    }

    // Inside your Bus/Memory write logic
    fn write_div(&mut self) {
        // 1. Check if the current bit being monitored by TAC is 1
        let bit_to_monitor = self.timer.get_tac_bit();
        let bit_was_high = (self.timer.internal_counter >> bit_to_monitor) & 0x01 == 1;
        let timer_enabled = self.timer.timer_enabled();

        // 2. Reset the counter
        self.timer.internal_counter = 0;
        self.timer.div = 0;

        // 3. Falling Edge Glitch:
        // If the bit was high and the timer was enabled,
        // resetting to 0 causes a falling edge!
        if timer_enabled && bit_was_high {
            self.timer.increment_tima();
        }
    }

    /// Returns true if a V-Blank is triggered.
    fn tick_components(&mut self, cycles: u8) -> bool {
        if self.timer.tick(cycles) {
            debug!("tick_components: timer interrup");
            // Bit 2 is the Timer Interrupt
            let interrupt_flags = self.read_byte(0xFF0F);
            self.write(0xFF0F, interrupt_flags | 0b100);
        }

        // You would also tick your PPU (Graphics) here later
        if self.ppu.tick(cycles) {
            // Manually trigger the V-Blank bit in the IF register (0xFF0F)
            let current_if = self.read_if();
            self.write_byte(0xFF0F, current_if | 0x01);
            return true;
        }
        false
    }
    /// @deprecated
    fn increment_cycles(&mut self, value: u64) {
        self.tick_components(value as u8);
    }
    fn write(&mut self, addr: u16, val: u8) {
        // $8000–$9FFF

        match addr {
            // 1. Handle Echo RAM Mirroring (0xE000 - 0xFDFF mirrors 0xC000 - 0xDFFF)
            0xE000..=0xFDFF => {
                let mirrored_addr = addr - 0x2000;
                self.data[mirrored_addr as usize] = val;
            }
            0x0000..=0x7FFF => { /* ROM - usually read only */ }
            0x8000..=0x9FFF => self.ppu.write_byte(addr, val), // VRAM
            0xFE00..=0xFE9F => self.ppu.write_byte(addr, val), // OAM
            0xFF00 => self.joypad_sel = val & 0x30,
            0xFF04 => self.write_div(), // Trigger the reset glitch
            0xFF05 => self.timer.tima = val,
            0xFF06 => self.timer.tma = val,
            0xFF46 => {
                // 1. The value written is the HIGH byte of the source address.
                // If val is 0xC0, source is 0xC000. If 0x80, source is 0x8000.
                let source_base = (val as u16) << 8;

                // 2. Perform the actual transfer of 160 bytes (OAM is 0xA0 bytes long)
                for i in 0..0xA0 {
                    // Read from the source address (could be ROM or RAM)
                    let data = self.read_byte(source_base + i);

                    // info!(
                    //     "write: 0xFF46: DMA Triggered! Source: {:#06X}, First Byte: {:#04X}",
                    //     source_base, data
                    // );

                    // Write it into the PPU's OAM memory
                    // OAM is 0xFE00 to 0xFE9F
                    self.ppu.write_oam(i as usize, data);
                }
            }
            0xFF40..=0xFF4B => self.ppu.write_byte(addr, val), // PPU Registers
            0xFF07 => {
                self.timer.write_tac(val);
            } // Trigger the TAC glitch
            _ => {}
        }

        if addr == 0xFF44 {
            // LY is read-only on real hardware.
            // Writing to it usually resets it to 0,
            // but for this hack, we just ignore the write.
        }

        // 2. Handle Serial/LY/other hooks here...
        // Hooks into Serial Port Link Cable interface.
        if addr == 0xFF02 && val == 0x81 {
            let c = self.read_byte(0xFF01) as char;
            print!("{}", c); // This prints test results like "CPU PASS"
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
        self.data[addr as usize] = val;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mmu::memory_trait::Memory;

    #[test]
    fn test_bus_timer_interrupt_integration() {
        let mut bus = Bus::new(Vec::new()); // Your memory/system component

        // 1. Configure Timer via Bus writes
        bus.write(0xFF06, 0xAA); // TMA = 170
        bus.write(0xFF07, 0x05); // TAC = Enabled, 16-cycle mode
        bus.write(0xFF05, 0xFE); // TIMA = 254

        // Clear Interrupt Flags
        bus.write(0xFF0F, 0x00);

        // 2. We need 32 T-cycles to trigger two increments (254 -> 255 -> 0/Reload)
        // If your bus.tick() takes M-cycles, divide by 4.
        // Assuming bus.tick(cycles) takes T-cycles here:
        for _ in 0..32 {
            bus.tick_components(1);
        }

        // 3. Verify the chain reaction
        let tima = bus.read_byte(0xFF05);
        let if_reg = bus.read_byte(0xFF0F);

        assert_eq!(tima, 0xAA, "TIMA should have reloaded from TMA (0xAA)");
        assert!(
            if_reg & 0x04 != 0,
            "Timer interrupt bit (2) should be set in IF register"
        );
    }
    #[test]
    fn test_timer_via_tick_components() {
        let mut bus = Bus::new(Vec::new());

        // Setup: Fast timer (16 cycle mode), enabled
        bus.write(0xFF07, 0x05);
        bus.write(0xFF06, 0xAA); // TMA = 0xAA
        bus.write(0xFF05, 0xFF); // TIMA = 0xFF (One step from overflow)
        bus.write(0xFF0F, 0x00); // Clear Interrupt Flags

        // Execute 16 cycles (enough for one TIMA increment at speed 01)
        bus.tick_components(16);

        // Verify
        let tima = bus.read_byte(0xFF05);
        let if_reg = bus.read_byte(0xFF0F);

        assert_eq!(
            tima, 0xAA,
            "TIMA should have wrapped around to TMA value 0xAA"
        );
        assert_eq!(
            if_reg & 0x04,
            0x04,
            "Timer interrupt bit (2) should be set in IF register"
        );
    }
    #[test]
    fn test_bus_div_reset_glitch() {
        let mut bus = Bus::new(Vec::new());

        bus.write(0xFF07, 0x05); // Enable, 16-cycle mode (Bit 3)
        bus.write(0xFF05, 0x00); // TIMA = 0

        // 1. Tick 8 times. Internal counter is 8 (0b1000). Bit 3 is HIGH.
        bus.tick_components(8);
        assert_eq!(
            bus.read_byte(0xFF05),
            0,
            "TIMA should not have incremented yet"
        );

        // 2. Write to DIV to reset it.
        // This should trigger the falling edge glitch and increment TIMA.
        bus.write(0xFF04, 0x00);

        assert_eq!(
            bus.read_byte(0xFF05),
            1,
            "TIMA should have incremented due to DIV reset glitch"
        );
    }
}
