pub mod terminal;

use core::fmt;

use log::{trace, warn};

const OAM_SIZE: usize = 0xA0; // 160 bytes.

pub trait Ppu {
    /// Advances the internal state machine by a number of T-cycles.
    /// Returns true if a V-Blank interrupt should be triggered.
    fn tick(&mut self, cycles: u8) -> bool;

    /// Read from PPU-owned memory (VRAM: 0x8000-0x9FFF, OAM: 0xFE00-0xFE9F, Registers: 0xFF40-0xFF4B)
    fn read_byte(&self, addr: u16) -> u8;

    /// Write to PPU-owned memory
    fn write_byte(&mut self, addr: u16, val: u8);

    /// Write directly to the OAM buffer.
    fn write_oam(&mut self, index: usize, val: u8);

    /// Returns the 160x144 pixel data.
    /// Using u8 to represent the 4 shades (0-3).
    fn get_frame_buffer(&self) -> &[u8; 23040];
    fn get_dot_counter(&self) -> u32;
    fn set_ly(&mut self, val: u8);
}

pub struct DummyPpu {
    pub vram: [u8; 0x2000], // 8KB
    pub oam: [u8; OAM_SIZE],
    pub ly: u8,           // Current Scanline (0xFF44)
    pub dot_counter: u32, // Progress within the current line
    pub frame_buffer: [u8; 160 * 144],
    pub lcdc: u8,
    pub scy: u8,
    pub scx: u8,
    pub bgp: u8,
    pub wx: u8,
    pub wy: u8,
    pub stat: u8,
    pub lyc: u8,
    pub obp0: u8,
    pub obp1: u8,
}

impl Default for DummyPpu {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyPpu {
    pub fn new() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; OAM_SIZE],
            ly: 0,
            dot_counter: 0,
            frame_buffer: [0; 160 * 144],
            lcdc: 0,
            scy: 0,
            scx: 0,
            bgp: 0,
            wx: 0,
            wy: 0,
            stat: 0,
            lyc: 0,
            obp0: 0,
            obp1: 0,
        }
    }

    fn render_line(&mut self) {
        let line = self.ly as usize;
        if line >= 144 {
            return;
        }

        // LCDC Bit 3: Background Tile Map Display Select (0=9800-9BFF, 1=9C00-9FFF)
        let tile_map_base = if (self.lcdc & 0x08) != 0 {
            0x9C00
        } else {
            0x9800
        };

        // LCDC Bit 4: Background & Window Tile Data Select (0=8800-97FF, 1=8000-8FFF)
        let bit4 = (self.lcdc & 0x10) != 0;

        for x in 0..160 {
            let tile_col = x / 8;
            let tile_row = line / 8;
            let tile_index_addr = tile_map_base + (tile_row as u16 * 32) + tile_col as u16;
            let tile_id = self.vram[(tile_index_addr - 0x8000) as usize];

            // Calculate tile data address based on bit 4
            let tile_addr = if bit4 {
                // Unsigned mode: Base is 0x8000
                0x8000 + (tile_id as u16 * 16)
            } else {
                // Signed mode: Base is 0x9000, tile_id is i8
                let offset = (tile_id as i8 as i16 * 16) as u16;
                0x9000_u16.wrapping_add(offset)
            };

            let pixel_row = (line % 8) as u16;
            let addr = tile_addr + (pixel_row * 2);

            let byte1 = self.vram[(addr - 0x8000) as usize];
            let byte2 = self.vram[(addr + 1 - 0x8000) as usize];

            let bit = 7 - (x % 8);
            let color_idx = (((byte2 >> bit) & 1) << 1) | ((byte1 >> bit) & 1);

            // Apply Background Palette (BGP - 0xFF47)
            let color = (self.bgp >> (color_idx * 2)) & 0b11;

            self.frame_buffer[line * 160 + x] = color;
        }
    }

    fn render_sprites(&mut self) {
        let ly = self.ly;
        // Check LCDC Bit 1: Are sprites even enabled?
        if (self.lcdc & 0x02) == 0 {
            return;
        }

        let mut sprites_on_line = 0;

        // Iterate through all 40 entries in OAM
        for i in 0..40 {
            let i = i * 4;
            let y_pos = self.oam[i].wrapping_sub(16);
            let x_pos = self.oam[i + 1].wrapping_sub(8);
            let tile_index = self.oam[i + 2];
            let attrs = self.oam[i + 3];

            // 1. Is the sprite on this specific scanline? (8px height)
            if ly >= y_pos && ly < y_pos.wrapping_add(8) {
                sprites_on_line += 1;
                if sprites_on_line > 10 {
                    break;
                } // Hardware limit

                // 2. Determine which row of the tile to draw
                let mut line_in_tile = ly.wrapping_sub(y_pos);

                // Check for Vertical Flip (Bit 6 of Attributes)
                if (attrs & 0x40) != 0 {
                    line_in_tile = 7 - line_in_tile;
                }

                // 3. Fetch tile data from VRAM ($8000 range)
                let data_addr = (tile_index as u16 * 16) + (line_in_tile as u16 * 2);
                let byte1 = self.vram[data_addr as usize];
                let byte2 = self.vram[(data_addr + 1) as usize];

                for x_offset in 0..8 {
                    let bit = 7 - x_offset;
                    let mut pixel_x = x_offset;

                    // Check for Horizontal Flip (Bit 5 of Attributes)
                    if (attrs & 0x20) != 0 {
                        pixel_x = 7 - x_offset;
                    }

                    let color_idx = ((byte1 >> bit) & 0x01) | (((byte2 >> bit) & 0x01) << 1);

                    // 4. Transparency Check: Sprite Color 0 is ALWAYS transparent
                    if color_idx == 0 {
                        continue;
                    }

                    let screen_x = x_pos.wrapping_add(pixel_x);
                    if screen_x < 160 {
                        // 5. Apply Palette (OBP0 or OBP1)
                        let palette = if (attrs & 0x10) != 0 {
                            self.obp1
                        } else {
                            self.obp0
                        };
                        let color = (palette >> (color_idx * 2)) & 0b11;

                        // 6. Draw to buffer (Handle Priority Bit 7 if needed)
                        self.frame_buffer[ly as usize * 160 + screen_x as usize] = color;
                    }
                }
            }
        }
    }

    // fn perform_dma(&mut self, source_high_byte: u8, bus: &dyn Memory) {
    //     let base_addr = (source_high_byte as u16) << 8;
    //     for i in 0..0xA0 {
    //         let val = bus.read_byte(base_addr + i);
    //         self.oam[i as usize] = val;
    //     }
    // }
    // fn perform_dma(&mut self, val: u8) {
    //     // The value written is the high byte of the source address (0xXX00)
    //     let source_base = (val as u16) << 8;

    //     for i in 0..0xA0 {
    //         // 160 bytes (40 sprites * 4 bytes each)
    //         // We read from the BUS because DMA can pull from ROM or RAM
    //         let data = self.read_byte(source_base + i);
    //         // We write directly to the PPU's OAM
    //         self.ppu.write_byte(0xFE00 + i, data);
    //     }
    // }
}

impl fmt::Debug for DummyPpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = self.stat & 0x03;
        let mode_str = match mode {
            0 => "H-Blank",
            1 => "V-Blank",
            2 => "OAM Scan",
            3 => "Drawing",
            _ => "Unknown",
        };

        write!(
            f,
            "--- PPU State ---\n\
             LY:   {:<3} (0x{:02X}) | LYC:  {:<3} (0x{:02X})\n\
             DOTS: {:<3}            | STAT: 0x{:02X} ({})\n\
             LCDC: 0x{:02X}         | BGP:  0x{:02X}\n\
             SCX:  0x{:02X}         | SCY:  0x{:02X}\n\
             WX:   0x{:02X}         | WY:   0x{:02X}\n\
             OBP0: 0x{:02X}         | OBP1: 0x{:02X}\n\
             -----------------",
            self.ly,
            self.ly,
            self.lyc,
            self.lyc,
            self.dot_counter,
            self.stat,
            mode_str,
            self.lcdc,
            self.bgp,
            self.scx,
            self.scy,
            self.wx,
            self.wy,
            self.obp0,
            self.obp1
        )
    }
}

impl Ppu for DummyPpu {
    fn set_ly(&mut self, val: u8) {
        self.ly = val;
    }

    fn get_dot_counter(&self) -> u32 {
        self.dot_counter
    }

    fn write_oam(&mut self, addr: usize, val: u8) {
        // debug_assert!(addr > OAM_SIZE, "Out of bound write_oam");
        self.oam[addr] = val;
    }
    fn get_frame_buffer(&self) -> &[u8; 23040] {
        &self.frame_buffer
    }
    fn tick(&mut self, cycles: u8) -> bool {
        self.dot_counter += cycles as u32;

        if self.dot_counter >= 456 {
            self.dot_counter -= 456;

            // Only render if we are on a visible line (0-143)
            if self.ly < 144 {
                self.render_line();
                self.render_sprites();
            }

            self.ly = (self.ly + 1) % 154;

            if self.ly == 144 {
                trace!(
                    "ppu: tick: {}, counter: {}, ly: {}!",
                    cycles, self.dot_counter, self.ly
                );
                return true;
            }
        }
        trace!(
            "ppu: tick: {}, counter: {}, ly: {}",
            cycles, self.dot_counter, self.ly
        );
        false
    }

    fn read_byte(&self, addr: u16) -> u8 {
        // info!("ppu: read_byte: addr: {:04X}, val: {:02X}", addr, val);
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            0xFF40 => self.lcdc,
            0xFF41 => self.stat,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly, // This is the one the CPU polls most often
            0xFF45 => self.lyc,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF, // Default for unmapped IO
        }
    }

    fn write_byte(&mut self, addr: u16, val: u8) {
        // info!("ppu: write_byte: addr: {:04X}, val: {:02X}", addr, val);
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = val,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = val,
            0xFF40 => self.lcdc = val,
            0xFF41 => self.stat = val,
            0xFF42 => self.scy = val,
            0xFF43 => self.scx = val,
            0xFF44 => self.ly = 0, // Writing to LY usually resets it on real hardware
            0xFF45 => self.lyc = val,
            // 0xFF46 => self.perform_dma(val),
            0xFF47 => self.bgp = val,
            0xFF48 => self.obp0 = val,
            0xFF49 => self.obp1 = val,
            0xFF4A => self.wy = val,
            0xFF4B => self.wx = val,
            _ => {
                // Log unhandled writes instead of panicking
                warn!("PPU: Unhandled Write_byte, addr: {addr:04X}, val: {val:02X}");
            }
        }
    }
}
