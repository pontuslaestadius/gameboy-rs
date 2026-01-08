/// Holds the content of the rom, As to load it in to memory.
#[derive(Debug, PartialEq)]
pub struct Rom {
    pub content: Vec<u8>,
}

impl Rom {
    pub fn new(content: Vec<u8>) -> Rom {
        // let mut data = Vec::new();
        Rom { content }
    }
}

// EEPROM
