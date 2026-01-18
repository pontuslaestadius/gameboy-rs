/*
parses ROM headers

Address Range,Name,Purpose
0x0100–0x0103,Entry Point,Usually contains a NOP followed by a JP 0x0150. This is the first code the CPU runs.
0x0104–0x0133,Nintendo Logo,"A bitmap of the Nintendo logo. The Boot ROM compares this to its own copy; if it doesn't match, the GB won't boot."
0x0134–0x0143,Title,Uppercase ASCII text of the game's name.
0x0144–0x0145,New Licensee Code,Two characters used to identify the game publisher.
0x0146,SGB Flag,Indicates if the game supports Super Game Boy features.
0x0147,Cartridge Type,Crucial: Tells you which MBC (if any) is inside the cart.
0x0148,ROM Size,Indicates how many banks the ROM has.
0x0149,RAM Size,Indicates how much external Save RAM is on the cart.
0x014A,Destination Code,Japanese vs. Non-Japanese market.
0x014B,Old Licensee Code,Older identification for publishers.
0x014C,Mask ROM Version,The version number of the game.
0x014D,Header Checksum,A checksum of bytes 0134–014C. The GB won't boot if this is wrong.
0x014E–0x014F,Global Checksum,A checksum of the entire ROM (the GB hardware doesn't actually check this).
*/

use log::{error, info};

#[derive(Debug, Default)]
pub struct Headers {
    pub title: Option<String>,
    pub licensee_new: u16,    // 0x0144-0x0145
    pub sgb_flag: u8,         // 0x0146
    pub cart_type: u8,        // 0x0147
    pub rom_size_raw: u8,     // 0x0148
    pub ram_size_raw: u8,     // 0x0149
    pub destination: u8,      // 0x014A
    pub licensee_old: u8,     // 0x014B
    pub version: u8,          // 0x014C
    pub checksum_header: u8,  // 0x014D
    pub checksum_global: u16, // 0x014E-0x014F
    pub is_valid: bool,       // Results of the internal checks
}

impl Headers {
    pub fn new(content: &[u8]) -> Self {
        if content.len() < 0x0150 {
            error!("ROM size {} too small for header", content.len());
            return Self::default();
        }

        let mut headers = Self {
            title: extract_title(content),
            licensee_new: u16::from_be_bytes([content[0x0144], content[0x0145]]),
            sgb_flag: content[0x0146],
            cart_type: content[0x0147],
            rom_size_raw: content[0x0148],
            ram_size_raw: content[0x0149],
            destination: content[0x014A],
            licensee_old: content[0x014B],
            version: content[0x014C],
            checksum_header: content[0x014D],
            checksum_global: u16::from_be_bytes([content[0x014E], content[0x014F]]),
            is_valid: false,
        };

        headers.is_valid = headers.validate(content);
        // info!("Cartridge headers: {:?}", headers);
        headers
    }

    fn validate(&self, content: &[u8]) -> bool {
        // 1. Nintendo Logo Check (The most famous hardware check)
        // Original hardware won't boot if this is wrong.
        if !verify_nintendo_logo(content) {
            error!("Nintendo Logo verification failed!");
            return false;
        }

        // 2. Header Checksum (0x014D)
        let mut x: u8 = 0;
        for i in 0x0134..=0x014C {
            x = x.wrapping_sub(content[i]).wrapping_sub(1);
        }

        if x != self.checksum_header {
            error!(
                "Header Checksum mismatch! Calculated: {:02X}, Header: {:02X}",
                x, self.checksum_header
            );
            return false;
        }

        true
    }

    pub fn rom_banks(&self) -> usize {
        // Spec: 32KB << rom_size_raw (where 0 is 32KB/2 banks)
        2 << self.rom_size_raw
    }
}

fn verify_nintendo_logo(content: &[u8]) -> bool {
    const NINTENDO_LOGO: [u8; 48] = [
        0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00,
        0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD,
        0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB,
        0xB9, 0x33, 0x3E,
    ];
    &content[0x0104..0x0134] == NINTENDO_LOGO
}

fn extract_title(content: &[u8]) -> Option<String> {
    let title_bytes = &content[0x0134..=0x0143];

    // Convert to string, but stop at the first NULL byte (0x00)

    let title =
        String::from_utf8_lossy(title_bytes.split(|&b| b == 0).next().unwrap_or(&[])).into_owned();

    Some(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A helper to create a "blank" valid header for testing
    fn create_valid_header_buffer() -> Vec<u8> {
        let mut buf = vec![0; 0x150];

        // 1. Nintendo Logo
        let logo = [
            0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C,
            0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6,
            0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC,
            0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
        ];
        buf[0x0104..0x0134].copy_from_slice(&logo);

        // 2. Title: "TETRIS"
        buf[0x0134..0x013A].copy_from_slice(b"TETRIS");

        // 3. Cart Type: MBC1 (0x01), ROM Size: 32KB (0x00)
        buf[0x0147] = 0x01;
        buf[0x0148] = 0x00;

        // 4. Calculate Header Checksum (crucial for is_valid to be true)
        let mut x: u8 = 0;
        for i in 0x0134..=0x014C {
            x = x.wrapping_sub(buf[i]).wrapping_sub(1);
        }
        buf[0x014D] = x;

        buf
    }

    #[test]
    fn test_valid_header_parsing() {
        let data = create_valid_header_buffer();
        let h = Headers::new(&data);

        assert!(h.is_valid);
        assert_eq!(h.title, Some("TETRIS".to_string()));
        assert_eq!(h.cart_type, 0x01);
        assert_eq!(h.rom_banks(), 2); // 32KB = 2 banks
    }

    #[test]
    fn test_too_small_buffer() {
        let data = vec![0x00; 0x100]; // Less than 0x150
        let h = Headers::new(&data);

        // Should return default and not panic
        assert!(!h.is_valid);
        assert_eq!(h.title, None);
    }

    #[test]
    fn test_invalid_checksum() {
        let mut data = create_valid_header_buffer();
        data[0x014D] = 0xFF; // Sabotage the checksum

        let h = Headers::new(&data);
        assert!(!h.is_valid); // Checksum should fail
    }

    #[test]
    fn test_corrupt_logo() {
        let mut data = create_valid_header_buffer();
        data[0x0104] = 0x00; // Change the first byte of the Nintendo logo

        let h = Headers::new(&data);
        assert!(!h.is_valid); // Logo check should fail
    }

    #[test]
    fn test_rom_bank_calculation() {
        let mut h = Headers::default();

        h.rom_size_raw = 0x00; // 32KB
        assert_eq!(h.rom_banks(), 2);

        h.rom_size_raw = 0x01; // 64KB
        assert_eq!(h.rom_banks(), 4);

        h.rom_size_raw = 0x05; // 1MB
        assert_eq!(h.rom_banks(), 64);
    }
}
