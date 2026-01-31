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
    constants::*,
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
    // TODO: make generic.
    pub ppu: Box<DummyPpu>,
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

    pub fn force_write_byte(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }
}

impl<I: InputDevice + Default> Memory for Bus<I> {
    fn read_byte(&self, addr: u16) -> u8 {
        let val = match addr {
            // ROM: 0x0000..=0x7FFF
            ADDR_MEM_ROM_START..=ADDR_MEM_ROM_END => {
                let b = self.data[addr as usize];
                trace!("read_byte ROM addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // VRAM: 0x8000..=0x9FFF
            ADDR_MEM_VRAM_START..=ADDR_MEM_VRAM_END => {
                let b = self.ppu.read_byte(addr);
                trace!("read_byte VRAM addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // WRAM: 0xC000..=0xDFFF
            ADDR_MEM_WRAM_START..=ADDR_MEM_WRAM_END => {
                let b = self.data[addr as usize];
                trace!("read_byte WRAM addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // Echo RAM: 0xE000..=0xFDFF
            ADDR_MEM_ECHO_START..=ADDR_MEM_ECHO_END => {
                let b = self.data[(addr - 0x2000) as usize];
                trace!("read_byte Echo RAM addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // OAM: 0xFE00..=0xFE9F
            ADDR_MEM_OAM_START..=ADDR_MEM_OAM_END => {
                let b = self.ppu.read_byte(addr);
                trace!("read_byte OAM addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // Forbidden Zone: 0xFEA0..=0xFEFF
            0xFEA0..=0xFEFF => {
                trace!(
                    "read_byte Forbidden Zone addr: {:#06X}, returning 0xFF",
                    addr
                );
                0xFF
            }

            // Joypad: 0xFF00
            ADDR_SYS_JOYP => {
                let mut result = 0xCF | self.joypad_sel;
                if (self.joypad_sel & 0x10) == 0 {
                    result &= self.input.read(self.joypad_sel);
                }
                if (self.joypad_sel & 0x20) == 0 {
                    result &= self.input.read(self.joypad_sel);
                }
                trace!("read_byte Joypad addr: {:#06X}, val: {:#04X}", addr, result);
                result
            }

            // Timer: 0xFF04..=0xFF07
            ADDR_TIMER_DIV..=ADDR_TIMER_TAC => {
                let b = self.timer.read_byte(addr);
                trace!("read_byte Timer addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // PPU Registers: 0xFF40..=0xFF4B
            ADDR_PPU_LCDC..=ADDR_PPU_WX => {
                let b = self.ppu.read_byte(addr);
                trace!("read_byte PPU Reg addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // High RAM (HRAM): 0xFF80..=0xFFFE
            ADDR_MEM_HRAM_START..=ADDR_MEM_HRAM_END => {
                let b = self.data[addr as usize];
                trace!("read_byte HRAM addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            ADDR_SYS_IE => {
                let b = self.data[addr as usize];
                trace!("read_byte IE addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            ADDR_SYS_IF => {
                let b = self.data[addr as usize];
                trace!("read_byte IF addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }

            // Default
            _ => {
                let b = self.data[addr as usize];
                trace!("read_byte Default addr: {:#06X}, val: {:#04X}", addr, b);
                b
            }
        };
        val
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
        // trace!("tick components");
        if self.timer.tick(cycles) {
            trace!("tick_components: timer interrup");
            // Bit 2 is the Timer Interrupt
            let interrupt_flags = self.read_if();
            self.write_if(interrupt_flags | 0b100);
        }

        // You would also tick your PPU (Graphics) here later
        if self.ppu.tick(cycles) {
            trace!("tick_components: ppu triggered V-Blank");
            // Manually trigger the V-Blank bit in the IF register (0xFF0F)
            let current_if = self.read_if();
            self.write_if(current_if | 0x01);
            return true;
        }
        false
    }
    fn write_byte(&mut self, addr: u16, val: u8) {
        match addr {
            // ROM: 0x0000..=0x7FFF (Read Only)
            ADDR_MEM_ROM_START..=ADDR_MEM_ROM_END => {
                trace!(
                    "write_byte [0x{:04X}] -> 0x{:02X} (IGNORED: ROM is Read Only)",
                    addr, val
                );
            }

            // VRAM: 0x8000..=0x9FFF
            ADDR_MEM_VRAM_START..=ADDR_MEM_VRAM_END => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (VRAM)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            // External RAM: 0xA000..=0xBFFF (Assuming simple mapping for now)
            ADDR_MEM_SRAM_START..=ADDR_MEM_SRAM_END => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (EXT RAM)", addr, val);
                self.data[addr as usize] = val;
            }

            // WRAM: 0xC000..=0xDFFF
            ADDR_MEM_WRAM_START..=ADDR_MEM_WRAM_END => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (WRAM)", addr, val);
                self.data[addr as usize] = val;
            }

            // Echo RAM: 0xE000..=0xFDFF
            ADDR_MEM_ECHO_START..=ADDR_MEM_ECHO_END => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (ECHO RAM)", addr, val);
                self.data[(addr - 0x2000) as usize] = val;
            }

            // OAM: 0xFE00..=0xFE9F
            ADDR_MEM_OAM_START..=ADDR_MEM_OAM_END => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (OAM)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            // Forbidden Zone: 0xFEA0..=0xFEFF
            0xFEA0..=0xFEFF => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (FORBIDDEN)", addr, val);
            }

            // Joypad: 0xFF00
            ADDR_SYS_JOYP => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (JOYPAD SEL)", addr, val);
                self.joypad_sel = val & 0x30;
            }

            // Serial Data & Control: 0xFF01..=0xFF02
            0xFF01 => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (SERIAL DATA)", addr, val);
                self.data[addr as usize] = val;
            }
            ADDR_SYS_SB => {
                // Assuming ADDR_SYS_SB is 0xFF02
                if val == 0x81 {
                    let c = self.data[0xFF01] as char;
                    trace!(
                        "write_byte [0x{:04X}] -> 0x{:02X} (SERIAL LOG: '{}')",
                        addr, val, c
                    );
                    print!("{}", c);
                    std::io::stdout().flush().unwrap();
                }
            }

            // Timer Registers: 0xFF04..=0xFF07
            ADDR_TIMER_DIV => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (DIV RESET)", addr, val);
                self.write_div();
            }
            ADDR_TIMER_TIMA => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (TIMA)", addr, val);
                self.timer.tima = val;
            }
            ADDR_TIMER_TMA => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (TMA)", addr, val);
                self.timer.tma = val;
            }
            ADDR_TIMER_TAC => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (TAC)", addr, val);
                self.timer.write_tac(val);
            }

            // DMA Transfer: 0xFF46
            ADDR_PPU_DMA => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (DMA)", addr, val);
                self.dma_transfer(val);
            }

            // PPU Registers: 0xFF40..=0xFF4B (excluding DMA)
            ADDR_PPU_LCDC..=ADDR_PPU_WX => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (PPU REG)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            // High RAM: 0xFF80..=0xFFFE
            ADDR_MEM_HRAM_START..=ADDR_MEM_HRAM_END => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (HRAM)", addr, val);
                self.data[addr as usize] = val;
            }

            // Interrupt Enable: 0xFFFF
            ADDR_SYS_IE => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (IE REG)", addr, val);
                self.data[addr as usize] = val;
            }

            _ => {
                trace!("write_byte [0x{:04X}] -> 0x{:02X} (GENERAL/IO)", addr, val);
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
    #[test]
    fn test_vram_bus_communication() {
        let mut bus = bus();
        let vram_addr = 0x8000;
        let test_byte = 0x55;

        // Write to VRAM via Bus
        bus.write_byte(vram_addr, test_byte);

        // Read from VRAM via Bus
        let read_val = bus.read_byte(vram_addr);

        assert_eq!(
            read_val, test_byte,
            "VRAM Read/Write mismatch! Wrote {:02X}, Read {:02X}. Check Bus routing for 0x8000..=0x9FFF",
            test_byte, read_val
        );
    }
    #[test]
    fn test_timer_standalone_lifecycle() {
        let mut bus = bus();

        // 1. Setup Timer: Enable, Speed 01 (16 T-cycles)
        bus.write_byte(0xFF07, 0x05);
        bus.write_byte(0xFF06, 0xAA); // TMA Reload value
        bus.write_byte(0xFF05, 0xFE); // TIMA start

        // Verify initial state
        assert_eq!(bus.read_byte(0xFF05), 0xFE);

        // 2. Tick exactly 15 cycles. TIMA should NOT change yet.
        bus.timer.tick(15);
        assert_eq!(
            bus.read_byte(0xFF05),
            0xFE,
            "TIMA should not increment until 16 cycles"
        );

        // 3. Tick the 16th cycle.
        bus.timer.tick(1);
        assert_eq!(
            bus.read_byte(0xFF05),
            0xFF,
            "TIMA should be 0xFF after exactly 16 cycles"
        );

        // 4. Tick 15 more cycles. Still 0xFF.
        bus.timer.tick(15);
        assert_eq!(
            bus.read_byte(0xFF05),
            0xFF,
            "TIMA should stay 0xFF until the next 16-cycle boundary"
        );

        // 5. Tick 1 cycle to trigger overflow (255 -> 0).
        bus.tick_components(1);

        // Check state immediately after overflow
        let tima_val = bus.read_byte(0xFF05);
        let if_reg = bus.read_if();

        // Depending on your implementation of the "4-cycle delay":
        // If you don't have a delay, this should be 0xAA and bit 2 of IF should be set.
        assert!(
            tima_val == 0x00 || tima_val == 0xAA,
            "TIMA overflowed but got 0x{:02X}",
            tima_val
        );

        // 6. Ensure reload and interrupt are processed
        bus.tick_components(4); // Extra ticks to clear any internal delay logic

        assert_eq!(
            bus.read_byte(0xFF05),
            0xAA,
            "TIMA should be reloaded with TMA (0xAA)"
        );
        assert!(
            bus.read_if() & 0x04 != 0,
            "Timer interrupt bit (2) should be set in IF"
        );
    }
    #[test]
    fn diagnostic_timer_signal() {
        let mut bus = bus();

        bus.write_byte(0xFF07, 0x05);
        bus.write_byte(0xFF05, 0xFF); // One tick away from overflow

        // 1. Trigger the overflow
        bus.tick_components(16);

        // 2. Check internal vs external state
        let external_if_reg = bus.read_byte(0xFF0F);

        println!("Bus IF Register: 0x{:02X}", external_if_reg);

        assert!(external_if_reg & 0x04 != 0, "IF register bit 2 is NOT set");
    }
}
