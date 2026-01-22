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

pub const ADDR_VBLANK: u16 = 0x0040;
pub const ADDR_LCD_STAT: u16 = 0x0048;
pub const ADDR_TIMER: u16 = 0x0050;
pub const ADDR_SERIAL: u16 = 0x0058;
pub const ADDR_JOYPAD: u16 = 0x0060;

pub const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706); // ~59.7 fps
