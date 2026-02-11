use std::time::Duration;

// Constants for flags
pub const FLAG_Z: u8 = 0b1000_0000;
pub const FLAG_N: u8 = 0b0100_0000;
pub const FLAG_H: u8 = 0b0010_0000;
pub const FLAG_C: u8 = 0b0001_0000;
/// https://8bitnotes.com/2017/05/z80-timing/
pub const T_CYCLE: std::time::Duration = std::time::Duration::from_nanos(250);

pub const CB_PREFIX_OPCODE_BYTE: u8 = 0xCB;

pub const GAME_BOY_FILE_EXT: &str = "gb";

pub const IF_ADDR: u16 = 0xFF0F;
pub const IE_ADDR: u16 = 0xFFFF;

pub const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706 * 10); // ~59.7 fps
// pub const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706); // ~59.7 fps

// --- System & Interrupts ---
pub const ADDR_SYS_JOYP: u16 = 0xFF00; // Joypad selection and button status
pub const ADDR_SYS_SB: u16 = 0xFF01; // Serial transfer data
pub const ADDR_SYS_SC: u16 = 0xFF02; // Serial transfer control
pub const ADDR_SYS_IF: u16 = 0xFF0F; // Interrupt Flag
pub const ADDR_SYS_IE: u16 = 0xFFFF; // Interrupt Enable

// --- Interrupt Vectors ---
pub const ADDR_VEC_VBLANK: u16 = 0x0040; // V-Blank Interrupt Vector
pub const ADDR_VEC_LCD_STAT: u16 = 0x0048; // LCD Stat Interrupt Vector
pub const ADDR_VEC_TIMER: u16 = 0x0050; // Timer Interrupt Vector
pub const ADDR_VEC_SERIAL: u16 = 0x0058; // Serial Interrupt Vector
pub const ADDR_VEC_JOYPAD: u16 = 0x0060; // Joypad Interrupt Vector

// --- Timer & Divider ---
pub const ADDR_TIMER_DIV: u16 = 0xFF04; // Divider Register
pub const ADDR_TIMER_TIMA: u16 = 0xFF05; // Timer Counter
pub const ADDR_TIMER_TMA: u16 = 0xFF06; // Timer Modulo
pub const ADDR_TIMER_TAC: u16 = 0xFF07; // Timer Control

// --- PPU Graphics ---
pub const ADDR_PPU_LCDC: u16 = 0xFF40; // LCD Control
pub const ADDR_PPU_STAT: u16 = 0xFF41; // LCD Status
pub const ADDR_PPU_SCY: u16 = 0xFF42; // Viewport Y
pub const ADDR_PPU_SCX: u16 = 0xFF43; // Viewport X
pub const ADDR_PPU_LY: u16 = 0xFF44; // Current Scanline
pub const ADDR_PPU_LYC: u16 = 0xFF45; // LY Compare
pub const ADDR_PPU_DMA: u16 = 0xFF46; // OAM DMA Source
pub const ADDR_PPU_BGP: u16 = 0xFF47; // Background Palette
pub const ADDR_PPU_OBP0: u16 = 0xFF48; // Object Palette 0
pub const ADDR_PPU_OBP1: u16 = 0xFF49; // Object Palette 1
pub const ADDR_PPU_WY: u16 = 0xFF4A; // Window Y
pub const ADDR_PPU_WX: u16 = 0xFF4B; // Window X

// --- Memory Regions (Bounds) ---
pub const ADDR_MEM_ROM_START: u16 = 0x0000; // Non-switchable ROM bank
pub const ADDR_MEM_ROM_END: u16 = 0x7FFF; // Switchable ROM bank end
pub const ADDR_MEM_VRAM_START: u16 = 0x8000; // Video RAM
pub const ADDR_MEM_VRAM_END: u16 = 0x9FFF; // 
pub const ADDR_MEM_SRAM_START: u16 = 0xA000; // External RAM (Cartridge)
pub const ADDR_MEM_SRAM_END: u16 = 0xBFFF; // 
pub const ADDR_MEM_WRAM_START: u16 = 0xC000; // Work RAM
pub const ADDR_MEM_WRAM_END: u16 = 0xDFFF; // 
pub const ADDR_MEM_ECHO_START: u16 = 0xE000; // Echo RAM (WRAM Mirror)
pub const ADDR_MEM_ECHO_END: u16 = 0xFDFF; // 
pub const ADDR_MEM_OAM_START: u16 = 0xFE00; // Sprite Attribute Table
pub const ADDR_MEM_OAM_END: u16 = 0xFE9F; // 
pub const ADDR_MEM_HRAM_START: u16 = 0xFF80; // High RAM
pub const ADDR_MEM_HRAM_END: u16 = 0xFFFE; // 

// --- Audio (APU) ---
pub const ADDR_APU_NR10: u16 = 0xFF10; // Sweep
pub const ADDR_APU_NR11: u16 = 0xFF11; // Length/Duty
pub const ADDR_APU_NR12: u16 = 0xFF12; // Envelope
pub const ADDR_APU_NR13: u16 = 0xFF13; // Freq Lo
pub const ADDR_APU_NR14: u16 = 0xFF14; // Freq Hi
pub const ADDR_APU_NR50: u16 = 0xFF24; // Volume/Vin
pub const ADDR_APU_NR51: u16 = 0xFF25; // Panning
pub const ADDR_APU_NR52: u16 = 0xFF26; // Status
pub const ADDR_APU_NR21: u16 = 0xFF16; // Ch 2 Length/Duty
pub const ADDR_APU_NR22: u16 = 0xFF17; // Ch 2 Envelope
pub const ADDR_APU_NR23: u16 = 0xFF18; // Ch 2 Freq Lo
pub const ADDR_APU_NR24: u16 = 0xFF19; // Ch 2 Freq Hi
pub const ADDR_APU_NR30: u16 = 0xFF1A; // Ch 3 On/Off
pub const ADDR_APU_NR31: u16 = 0xFF1B; // Ch 3 Length
pub const ADDR_APU_NR32: u16 = 0xFF1C; // Ch 3 Output Level
pub const ADDR_APU_NR33: u16 = 0xFF1D; // Ch 3 Freq Lo
pub const ADDR_APU_NR34: u16 = 0xFF1E; // Ch 3 Freq Hi
pub const ADDR_APU_NR41: u16 = 0xFF20; // Ch 4 Length
pub const ADDR_APU_NR42: u16 = 0xFF21; // Ch 4 Envelope
pub const ADDR_APU_NR43: u16 = 0xFF22; // Ch 4 Polynomial Counter
pub const ADDR_APU_NR44: u16 = 0xFF23; // Ch 4 Counter/Consecutive
pub const ADDR_APU_WAVE_START: u16 = 0xFF30; // Wave Pattern RAM Start
pub const ADDR_APU_WAVE_END: u16 = 0xFF3F; // Wave Pattern RAM End
