pub mod terminal;

use crate::constants::*;
use core::fmt;
use log::{trace, warn};

const OAM_SIZE: usize = 0xA0; // 160 bytes.

pub struct Ppu {
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
    pub stat_line: bool,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
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
            stat_line: false,
        }
    }

    pub fn enable_ldc(&mut self) {
        self.write_byte(ADDR_PPU_LCDC, 0x80);
    }

    pub fn lcd_enabled(&self) -> bool {
        // Bit 7 controls the LCD power
        (self.lcdc & 0x80) != 0
    }

    pub fn request_stat_interrupt(&mut self) {
        todo!("");
    }

    pub fn init_post_boot(&mut self) {
        // LCDC: 0x91 (LCD ON, Window Tile Map 0x9800, BG/Window Tile Data 0x8000, BG ON)
        self.write_byte(ADDR_PPU_LCDC, 0x91);

        // STAT: 0x85 (Bit 7 always 1, Bit 2 LYC=LY coincidence, Bit 0-1 Mode 1 V-Blank)
        // Note: Some logs expect 0x80 or 0x82, but 0x85 is common post-bootrom.
        self.stat = 0x85;

        // LY: 0x00
        self.ly = 0x00;

        // LYC: 0x00
        self.lyc = 0x00;

        // Palettes: Standard mapping (3, 2, 1, 0)
        self.write_byte(ADDR_PPU_BGP, 0xFC);
        self.write_byte(ADDR_PPU_OBP0, 0xFF);
        self.write_byte(ADDR_PPU_OBP1, 0xFF);

        // Scroll positions
        self.scy = 0x00;
        self.scx = 0x00;
        self.wy = 0x00;
        self.wx = 0x00;

        // Internal counters
        self.dot_counter = 0;
        self.stat_line = false;
    }

    // Inside PPU tick
    pub fn update_stat_interrupt(&mut self) -> bool {
        let mode = self.stat & 0x03;

        let lyc_int = (self.stat & 0x40) != 0 && (self.stat & 0x04) != 0;
        let mode2_int = (self.stat & 0x20) != 0 && mode == 2;
        let mode1_int = (self.stat & 0x10) != 0 && mode == 1;
        let mode0_int = (self.stat & 0x08) != 0 && mode == 0;

        let current_signal = lyc_int || mode2_int || mode1_int || mode0_int;

        // Detect Rising Edge
        let interrupt_triggered = !self.stat_line && current_signal;
        self.stat_line = current_signal;
        // println!(
        //     "update_stat_interrupt: mode: {mode}, lyc_int: {lyc_int}, mode2_int: {mode2_int}, mode1_int: {mode1_int}, mode0_int: {mode0_int}, current_signal: {current_signal}, interrupt_triggered: {interrupt_triggered}"
        // );

        interrupt_triggered
    }

    pub fn update_lyc(&mut self) {
        // 1. Check if LY matches LYC
        if self.ly == self.lyc {
            // 2. Set Bit 2 of STAT (The LYC == LY flag)
            self.stat |= 0x04;

            // 3. Trigger Interrupt if Bit 6 of STAT (LYC Interrupt Source) is enabled
            if (self.stat & 0x40) != 0 {
                self.request_stat_interrupt();
            }
        } else {
            // 4. Clear Bit 2 if they don't match
            self.stat &= !0x04;
        }
    }

    pub fn set_mode(&mut self, mode: u8) {
        // 1. Clear the old mode (bits 0 and 1)
        // 2. Set the new mode
        self.stat = (self.stat & !0x03) | (mode & 0x03);

        // 3. Handle STAT Interrupts
        // Most developers check for interrupts here.
        // Example: If mode is 0 (H-Blank) and bit 3 of STAT is 1,
        // you would trigger the LCD_STAT interrupt.
    }

    pub fn render_line(&mut self) {
        let line = self.ly;
        if line >= 144 {
            return;
        }

        // 1. Calculate the actual vertical position in the 256px background map
        let y_pos = line.wrapping_add(self.scy);
        let tile_row = (y_pos / 8) as u16;
        let pixel_row = (y_pos % 8) as u16;

        let tile_map_base = if (self.lcdc & 0x08) != 0 {
            0x9C00
        } else {
            0x9800
        };
        let bit4 = (self.lcdc & 0x10) != 0;
        // let bit4 = true; // Force unsigned mode

        for x in 0..160u8 {
            // 2. Calculate the actual horizontal position in the 256px map
            let x_pos = x.wrapping_add(self.scx);
            let tile_col = (x_pos / 8) as u16;
            let pixel_col = (x_pos % 8) as u32;

            // 3. Find the tile ID in the map
            let tile_index_addr = tile_map_base + (tile_row * 32) + tile_col;
            let tile_id = self.vram[(tile_index_addr - 0x8000) as usize];

            // 4. Calculate tile data address (Signed vs Unsigned)
            let tile_addr = if bit4 {
                0x8000 + (tile_id as u16 * 16)
            } else {
                let offset = (tile_id as i8 as i16 * 16) as u16;
                0x9000_u16.wrapping_add(offset)
            };

            // 5. Fetch the two bytes for the specific row of the tile
            let addr = tile_addr + (pixel_row * 2);
            let byte1 = self.vram[(addr - 0x8000) as usize];
            let byte2 = self.vram[(addr + 1 - 0x8000) as usize];

            // 6. Extract the color index for the specific pixel
            let bit = 7 - pixel_col;
            let color_idx = (((byte2 >> bit) & 1) << 1) | ((byte1 >> bit) & 1);

            // 7. Apply Background Palette
            let color = (self.bgp >> (color_idx * 2)) & 0b11;

            // Store color (0-3) in frame buffer
            self.frame_buffer[line as usize * 160 + x as usize] = color;
        }
    }

    pub fn render_sprites(&mut self) {
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
    pub fn set_ly(&mut self, val: u8) {
        self.ly = val;
    }

    pub fn get_dot_counter(&self) -> u32 {
        self.dot_counter
    }

    pub fn write_oam(&mut self, addr: usize, val: u8) {
        // debug_assert!(addr > OAM_SIZE, "Out of bound write_oam");
        self.oam[addr] = val;
    }
    pub fn get_frame_buffer(&self) -> &[u8; 23040] {
        &self.frame_buffer
    }

    pub fn tick(&mut self, cycles: u32) -> (bool, bool) {
        if !self.lcd_enabled() {
            trace!("ppu timer tick ignored, LCD disabled");
            return (false, false);
        }

        let mut vblank_triggered = false;
        let mut stat_triggered = false;

        for _ in 0..cycles {
            self.dot_counter += 1;

            // --- 1. Line Timing ---
            if self.dot_counter >= 456 {
                self.dot_counter = 0;
                self.ly = (self.ly + 1) % 154;
                self.update_lyc(); // Update Bit 2 of STAT

                if self.ly == 144 {
                    vblank_triggered = true;
                }
            }

            // --- 2. Mode Determination ---
            let new_mode = if self.ly >= 144 {
                1 // V-Blank
            } else {
                match self.dot_counter {
                    0..=79 => 2,   // OAM Search
                    80..=251 => 3, // Data Transfer (Approx)
                    _ => 0,        // H-Blank
                }
            };

            let old_mode = self.stat & 0x03;
            if old_mode == 3 && new_mode == 0 {
                self.render_line();
            }

            // --- 3. STAT Mode Update ---
            // Only update bits 0-1
            self.stat = (self.stat & !0x03) | (new_mode & 0x03);

            // --- 4. STAT Interrupt (Rising Edge) ---
            if self.update_stat_interrupt() {
                stat_triggered = true;
            }
        }

        (vblank_triggered, stat_triggered)
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            ADDR_PPU_LCDC => self.lcdc,
            ADDR_PPU_STAT => self.stat,
            ADDR_PPU_SCY => self.scy,
            ADDR_PPU_SCX => self.scx,
            ADDR_PPU_LY => self.ly, // This is the one the CPU polls most often
            ADDR_PPU_LYC => self.lyc,
            ADDR_PPU_BGP => self.bgp,
            ADDR_PPU_OBP0 => self.obp0,
            ADDR_PPU_OBP1 => self.obp1,
            ADDR_PPU_WY => self.wy,
            ADDR_PPU_WX => self.wx,
            _ => 0xFF, // Default for unmapped IO
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        // info!("ppu: write_byte: addr: {:04X}, val: {:02X}", addr, val);
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = val,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = val,
            0xFF40 => {
                let was_on = (self.lcdc & 0x80) != 0;
                let is_on = (val & 0x80) != 0;

                self.lcdc = val;

                if !was_on && is_on {
                    // LCD turned ON: Synchronization Point
                    self.dot_counter = 0;
                    self.ly = 0;

                    // Immediately enter Mode 2 (OAM Search)
                    // Set bits 0-1 of STAT to 0b10 (2)
                    self.stat = (self.stat & !0x03) | 0x02;
                } else if was_on && !is_on {
                    // LCD turned OFF: Reset state
                    self.dot_counter = 0;
                    self.ly = 0;
                    // Mode 0 (H-Blank) is the standard state when OFF
                    self.stat &= !0x03;
                }
            }
            0xFF41 => {
                // Bits 0, 1, 2 are Read-Only (Mode and LYC=LY flag)
                // Bit 7 is unused (usually returns 1)
                // Only bits 3, 4, 5, 6 are writable (Interrupt enabled)
                let writable_mask = 0b0111_1000;
                self.stat = (val & writable_mask) | (self.stat & !writable_mask) | 0x80;

                // Crucial: A write to STAT can trigger an interrupt if a condition is met
                self.update_stat_interrupt();
            }

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

impl fmt::Debug for Ppu {
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
            // This may look odd here, but in the terminal they are aligned.
            "--- PPU State ---------------------------------------------\n\
             LY:   0x{:02X} | LYC:  0x{:02X} | DOTS: 0x{:02X} | STAT: 0x{:02X} ({})\n\
             LCDC: 0x{:02X} | BGP:  0x{:02X} | SCX:  0x{:02X} | SCY:  0x{:02X}\n\
             WX:   0x{:02X} | WY:   0x{:02X} | OBP0: 0x{:02X} | OBP1: 0x{:02X}",
            self.ly,
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
