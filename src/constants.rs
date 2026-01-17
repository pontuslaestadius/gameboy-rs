// Constants for flags
pub const FLAG_Z: u8 = 0b1000_0000;
pub const FLAG_N: u8 = 0b0100_0000;
pub const FLAG_H: u8 = 0b0010_0000;
pub const FLAG_C: u8 = 0b0001_0000;
/// https://8bitnotes.com/2017/05/z80-timing/
pub const T_CYCLE: std::time::Duration = std::time::Duration::from_nanos(250);

pub const CB_PREFIX_OPCODE_BYTE: u8 = 0xCB;

pub const GAME_BOY_FILE_EXT: &str = "gb";
