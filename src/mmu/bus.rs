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

use log::{debug, trace};
use std::io::Write;

use crate::{
    input::InputDevice,
    mmu::memory_trait::Memory,
    ppu::{DummyPpu, Ppu},
    timer::Timer,
};

/// 64 Kb - The standard Game Boy address space
const MEMORY_SIZE: usize = 1024 * 64;

pub struct Bus<I: InputDevice + Default> {
    timer: Timer,
    // Must use a Vec since an Array would use the stack, and crash the application.
    // Using the heap is required.
    // rom_size: usize,
    // This puts exactly 64KB on the HEAP, not the STACK
    pub data: Box<[u8; MEMORY_SIZE]>,
    // total_cycles: u64,
    pub ppu: Box<dyn Ppu>,
    input: I,

    pub joypad_sel: u8,
}

impl<I: InputDevice + Default> Bus<I> {
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
            // rom_size,
            data: buffer,
            ppu: Box::new(DummyPpu::new()),
            joypad_sel: 0xFF,
            input: I::default(),
        }
    }
    fn dma_transfer(&mut self, val: u8) {
        // The value written is the high byte of the source (e.g., 0xC0 -> 0xC000)
        let source_base = (val as u16) << 8;

        for i in 0..160 {
            // We use read_byte to ensure we respect the memory map
            // (the data could be coming from ROM or WRAM)
            let data = self.read_byte(source_base + i);

            // We push the data directly into the PPU's OAM
            self.ppu.write_oam(i as usize, data);
        }
    }

    #[cfg(test)]
    pub fn force_write_byte(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }
}

impl<I: InputDevice + Default> Memory for Bus<I> {
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
            0xFF00 => {
                let mut result = 0xCF | self.joypad_sel; // Keep selection bits

                // If Bit 4 is 0, CPU is reading Buttons (A, B, Select, Start)
                if (self.joypad_sel & 0x10) == 0 {
                    // We use the same mask, but the Game Boy separates them by selection
                    // Start/A/B/Select are bits 0-3
                    result &= self.input.read(self.joypad_sel);
                }

                // If Bit 5 is 0, CPU is reading Directions (Right, Left, Up, Down)
                if (self.joypad_sel & 0x20) == 0 {
                    // Directions are also bits 0-3 when read from this register
                    result &= self.input.read(self.joypad_sel);
                }

                result
            }

            0xFF04..=0xFF07 => self.timer.read_byte(addr),
            // Handled by CPU.
            // 0xFF0F => self.interrupt_flags,
            // 0xFF0F => panic!("interrupt flags are handled internally by the CPU"),

            // PPU Registers
            0xFF40..=0xFF4B => self.ppu.read_byte(addr),

            // High RAM (HRAM)
            // 0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],

            // Interrupt Enable, handled by CPU.
            // 0xFFFF => self.interrupt_enable,
            // 0xFFFF => panic!("interrupt enable handled internally by the CPU"),

            // Default: Return data from the main block or 0xFF
            _ => self.data[addr as usize],
        }
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
            trace!("tick_components: timer interrup");
            // Bit 2 is the Timer Interrupt
            let interrupt_flags = self.read_byte(0xFF0F);
            self.write_byte(0xFF0F, interrupt_flags | 0b100);
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
    fn write_byte(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x7FFF => {
                // This is likely the cause of your test failures!
                trace!(
                    "write_byte [0x{:04X}] -> 0x{:02X} (IGNORED: ROM is Read Only)",
                    addr, val
                );
            }
            0x8000..=0x9FFF => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (VRAM)", addr, val);
                self.ppu.write_byte(addr, val);
            }
            0xFE00..=0xFE9F => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (OAM)", addr, val);
                self.ppu.write_byte(addr, val);
            }
            0xFF00 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (JOYPAD SEL)", addr, val);
                self.joypad_sel = val & 0x30;
            }
            0xFF04 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (DIV RESET)", addr, val);
                self.write_div();
            }
            0xFF05 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (TIMA)", addr, val);
                self.timer.tima = val;
            }
            0xFF06 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (TMA)", addr, val);
                self.timer.tma = val;
            }
            0xFF07 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (TAC)", addr, val);
                self.timer.write_tac(val);
            }
            0xFF46 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (DMA)", addr, val);
                self.dma_transfer(val);
            }
            0xFF40..=0xFF4B => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (PPU REG)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            0xE000..=0xFDFF => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (ECHO RAM)", addr, val);
                self.data[(addr - 0x2000) as usize] = val;
            }

            0xFF02 if val == 0x81 => {
                let c = self.read_byte(0xFF01) as char;
                trace!(
                    "write_byte [0x{:04X}] -> 0x{:02X} (SERIAL LOG: '{}')",
                    addr, val, c
                );
                print!("{}", c);
                std::io::stdout().flush().unwrap();
            }

            _ => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (GENERAL)", addr, val);
                self.data[addr as usize] = val;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::input::DummyInput;
    use crate::mmu::memory_trait::Memory;

    fn bus() -> Bus<DummyInput> {
        let bus: Bus<DummyInput> = Bus::new(Vec::new()); // Your memory/system component
        bus
    }

    #[test]
    fn test_bus_timer_interrupt_integration() {
        let mut bus = bus();

        // 1. Configure Timer via Bus writes
        bus.write_byte(0xFF06, 0xAA); // TMA = 170
        bus.write_byte(0xFF07, 0x05); // TAC = Enabled, 16-cycle mode
        bus.write_byte(0xFF05, 0xFE); // TIMA = 254

        // Clear Interrupt Flags
        bus.write_byte(0xFF0F, 0x00);

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
        let mut bus = bus();

        // Setup: Fast timer (16 cycle mode), enabled
        bus.write_byte(0xFF07, 0x05);
        bus.write_byte(0xFF06, 0xAA); // TMA = 0xAA
        bus.write_byte(0xFF05, 0xFF); // TIMA = 0xFF (One step from overflow)
        bus.write_byte(0xFF0F, 0x00); // Clear Interrupt Flags

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
        let mut bus = bus();

        bus.write_byte(0xFF07, 0x05); // Enable, 16-cycle mode (Bit 3)
        bus.write_byte(0xFF05, 0x00); // TIMA = 0

        // 1. Tick 8 times. Internal counter is 8 (0b1000). Bit 3 is HIGH.
        bus.tick_components(8);
        assert_eq!(
            bus.read_byte(0xFF05),
            0,
            "TIMA should not have incremented yet"
        );

        // 2. Write to DIV to reset it.
        // This should trigger the falling edge glitch and increment TIMA.
        bus.write_byte(0xFF04, 0x00);

        assert_eq!(
            bus.read_byte(0xFF05),
            1,
            "TIMA should have incremented due to DIV reset glitch"
        );
    }
}
