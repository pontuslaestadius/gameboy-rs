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

use log::{debug, info, trace};
// use std::io::Write;

use crate::{
    apu::Apu,
    constants::*,
    input::InputDevice,
    mmu::memory_trait::Memory,
    ppu::{DummyPpu, Ppu},
    timer::Timer,
};

/// 64 Kb - The standard Game Boy address space
const MEMORY_SIZE: usize = 1024 * 64;

pub struct Bus<I: InputDevice + Default> {
    pub timer: Timer,
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
    pub serial_buffer: Vec<u8>,
    pub apu: Apu,
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
            serial_buffer: Vec::new(),
            apu: Apu::new(),
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
}

impl<I: InputDevice + Default> Memory for Bus<I> {
    #[inline]
    fn force_write_byte(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }
    fn read_byte_raw(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    fn read_byte(&self, addr: u16) -> u8 {
        let val = match addr {
            // TODO: We just need to do this reliably somehow...
            0xFF44 => 0x90,
            // ROM: 0x0000..=0x7FFF
            ADDR_MEM_ROM_START..=ADDR_MEM_ROM_END => {
                let b = self.data[addr as usize];
                trace!("read [{:#06X}] -> {:#04X} (ROM)", addr, b);
                b
            }

            // VRAM: 0x8000..=0x9FFF
            ADDR_MEM_VRAM_START..=ADDR_MEM_VRAM_END => {
                let b = self.ppu.read_byte(addr);
                trace!("read [{:#06X}] -> {:#04X} (VRAM)", addr, b);
                b
            }

            // WRAM: 0xC000..=0xDFFF
            ADDR_MEM_WRAM_START..=ADDR_MEM_WRAM_END => {
                let b = self.data[addr as usize];
                trace!("read [{:#06X}] -> {:#04X} (WRAM)", addr, b);
                b
            }

            // Echo RAM: 0xE000..=0xFDFF
            ADDR_MEM_ECHO_START..=ADDR_MEM_ECHO_END => {
                let b = self.data[(addr - 0x2000) as usize];
                trace!("read [{:#06X}] -> {:#04X} (ECHO)", addr, b);
                b
            }

            // OAM: 0xFE00..=0xFE9F
            ADDR_MEM_OAM_START..=ADDR_MEM_OAM_END => {
                let b = self.ppu.read_byte(addr);
                trace!("read [{:#06X}] -> {:#04X} (OAM)", addr, b);
                b
            }

            // Forbidden Zone: 0xFEA0..=0xFEFF
            0xFEA0..=0xFEFF => {
                trace!("read {:#06X} -> 0xFF (FORBIDDEN)", addr);
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
                trace!("read [{:#06X}] -> {:#04X} (JOYPAD)", addr, result);
                result
            }

            // Timer: 0xFF04..=0xFF07
            ADDR_TIMER_DIV..=ADDR_TIMER_TAC => {
                let b = self.timer.read_byte(addr);
                trace!("read [{:#06X}] -> {:#04X} (TIMER)", addr, b);
                b
            }

            // PPU Registers: 0xFF40..=0xFF4B
            ADDR_PPU_LCDC..=ADDR_PPU_WX => {
                let b = self.ppu.read_byte(addr);
                trace!("read [{:#06X}] -> {:#04X} (PPU)", addr, b);
                b
            }

            // High RAM (HRAM): 0xFF80..=0xFFFE
            ADDR_MEM_HRAM_START..=ADDR_MEM_HRAM_END => {
                let b = self.data[addr as usize];
                trace!("read [{:#06X}] -> {:#04X} (HRAM)", addr, b);
                b
            }

            ADDR_SYS_IE => {
                let b = self.data[addr as usize];
                trace!("read [{:#06X}] -> {:#04X} (IE)", addr, b);
                b
            }

            ADDR_SYS_IF => {
                let b = self.data[addr as usize];
                trace!("read [{:#06X}] -> {:#04X} (IF)", addr, b);
                b
            }

            // Audio
            0xFF10..=0xFF3F => self.apu.read_byte(addr),

            // Default
            _ => {
                let b = self.data[addr as usize];
                trace!("read [{:#06X}] -> {:#04X} (DEFAULT)", addr, b);
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
                    "write [0x{:04X}] -> 0x{:02X} (IGNORED: ROM is Read Only)",
                    addr, val
                );
            }

            // VRAM: 0x8000..=0x9FFF
            ADDR_MEM_VRAM_START..=ADDR_MEM_VRAM_END => {
                trace!("write [0x{:04X}] <- 0x{:02X} (VRAM)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            // External RAM: 0xA000..=0xBFFF (Assuming simple mapping for now)
            ADDR_MEM_SRAM_START..=ADDR_MEM_SRAM_END => {
                trace!("write [0x{:04X}] <- 0x{:02X} (EXT RAM)", addr, val);
                self.data[addr as usize] = val;
            }

            // WRAM: 0xC000..=0xDFFF
            ADDR_MEM_WRAM_START..=ADDR_MEM_WRAM_END => {
                trace!("write [0x{:04X}] <- 0x{:02X} (WRAM)", addr, val);
                self.data[addr as usize] = val;
            }

            // Echo RAM: 0xE000..=0xFDFF
            ADDR_MEM_ECHO_START..=ADDR_MEM_ECHO_END => {
                trace!("write [0x{:04X}] <- 0x{:02X} (ECHO RAM)", addr, val);
                self.data[(addr - 0x2000) as usize] = val;
            }

            // OAM: 0xFE00..=0xFE9F
            ADDR_MEM_OAM_START..=ADDR_MEM_OAM_END => {
                trace!("write [0x{:04X}] <- 0x{:02X} (OAM)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            // Forbidden Zone: 0xFEA0..=0xFEFF
            0xFEA0..=0xFEFF => {
                trace!("write [0x{:04X}] -- 0x{:02X} (FORBIDDEN)", addr, val);
            }

            // Joypad: 0xFF00
            ADDR_SYS_JOYP => {
                trace!("write [0x{:04X}] <- 0x{:02X} (JOYPAD SEL)", addr, val);
                self.joypad_sel = val & 0x30;
            }

            // Serial Data & Control: 0xFF01..=0xFF02
            ADDR_SYS_SB => {
                trace!("write [0x{:04X}] <- 0x{:02X} (SERIAL DATA)", addr, val);
                self.data[addr as usize] = val;
            }
            ADDR_SYS_SC => {
                if val == 0x81 {
                    // Transfer requested! Grab the byte from SB.
                    let c = self.data[ADDR_SYS_SB as usize];
                    self.serial_buffer.push(c);
                    // This comes with the defect of printing a new line
                    // between each character. We'll just pretend it's
                    // intentional,as it's Japanese hardware.
                    // :
                    // )
                    // info!("{}", c);
                    // Flush is only required if we're using print.
                    // let _ = std::io::stdout().flush();

                    // On real hardware, the bit 7 is cleared after transfer completes.
                    // Some test ROMs wait for this bit to clear.
                    self.data[ADDR_SYS_SC as usize] = val & 0x7F;
                } else {
                    self.data[ADDR_SYS_SC as usize] = val;
                }
            }

            // Timer Registers: 0xFF04..=0xFF07
            ADDR_TIMER_DIV => {
                trace!("write [0x{:04X}] <- 0x{:02X} (DIV RESET)", addr, val);
                self.write_div();
            }
            ADDR_TIMER_TIMA => {
                trace!("write [0x{:04X}] <- 0x{:02X} (TIMA)", addr, val);
                self.timer.tima = val;
            }
            ADDR_TIMER_TMA => {
                trace!("write [0x{:04X}] <- 0x{:02X} (TMA)", addr, val);
                self.timer.tma = val;
            }
            ADDR_TIMER_TAC => {
                trace!("write [0x{:04X}] <- 0x{:02X} (TAC)", addr, val);
                self.timer.write_tac(val);
            }

            // DMA Transfer: 0xFF46
            ADDR_PPU_DMA => {
                trace!("write [0x{:04X}] <- 0x{:02X} (DMA)", addr, val);
                self.dma_transfer(val);
            }

            // PPU Registers: 0xFF40..=0xFF4B (excluding DMA)
            ADDR_PPU_LCDC..=ADDR_PPU_WX => {
                trace!("write [0x{:04X}] <- 0x{:02X} (PPU REG)", addr, val);
                self.ppu.write_byte(addr, val);
            }

            // High RAM: 0xFF80..=0xFFFE
            ADDR_MEM_HRAM_START..=ADDR_MEM_HRAM_END => {
                trace!("write [0x{:04X}] <- 0x{:02X} (HRAM)", addr, val);
                self.data[addr as usize] = val;
            }

            // Interrupt Enable: 0xFFFF
            ADDR_SYS_IE => {
                trace!("write [0x{:04X}] <- 0x{:02X} (IE REG)", addr, val);
                self.data[addr as usize] = val;
            }

            // Audio
            0xFF10..=0xFF3F => self.apu.write_byte(addr, val),

            _ => {
                trace!("write [0x{:04X}] <- 0x{:02X} (GENERAL/IO)", addr, val);
                self.data[addr as usize] = val;
            }
        }
    }
}
