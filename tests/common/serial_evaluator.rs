use std::process::exit;

use gameboy_rs::{cpu::Cpu, input::DummyInput, mmu::Bus, ppu::Ppu};

use crate::common::EvaluationSpec;

const PASSED_STR: &[u8] = b"Passed";
const FAILED_STR: &[u8] = b"Failed";

pub struct SerialEvaluator {
    max_cycles: u64,
    cycles: u64,
}

impl SerialEvaluator {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            // For debug: this may take ~10s.
            // For release: ~0.7s before timeout.
            // Unfortunately, this should be ran in release mode
            // Otherwise it would take ages.
            max_cycles: 5_000_000,
        }
    }
}

impl EvaluationSpec for SerialEvaluator {
    fn evaluate(&mut self, _cpu: &Cpu, bus: &mut Bus<DummyInput>) -> bool {
        self.cycles += 1;
        // Check the display buffer ever so often, it's a pricier comparison,
        // so we only do it with some infrequency for performance.
        if self.cycles % 100_000 == 0 {
            let display_str = scrape_test_result(&*bus.ppu);
            let bytes = display_str.as_bytes();
            if contains_bytes(bytes, PASSED_STR) || contains_bytes(bytes, FAILED_STR) {
                return false;
            }
        }

        if let Some(serial_buffer) = bus.read_if_dirty_serial_buffer() {
            let len = serial_buffer.len();
            if len > 5 {
                // Check the last 100 bytes (or the whole buffer if smaller)
                // to avoid re-scanning the entire history every time.
                let scan_range = if len > 100 {
                    &serial_buffer[len - 100..]
                } else {
                    &serial_buffer[..]
                };

                if contains_bytes(scan_range, PASSED_STR) || contains_bytes(scan_range, FAILED_STR)
                {
                    return false;
                }
            }
        }

        self.cycles < self.max_cycles
    }

    fn report(&self, _cpu: &Cpu, bus: &Bus<DummyInput>) {
        let output = String::from_utf8_lossy(&bus.serial_buffer);
        if !output.contains("Passed") {
            if !output.is_empty() {
                println!("output: {}", output);
            }
            println!("Cycles: {}/{}", self.cycles, self.max_cycles);
            let display_str = scrape_test_result(&*bus.ppu);
            if !display_str.is_empty() {
                if display_str.contains("Passed") {
                    exit(0);
                }
                println!("--- Display --------");
                println!("{}", display_str);
                // println!("--------------------");
            }
            exit(1);
        }
        exit(0);
    }
}

// Helper to check for sub-slices
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub fn scrape_test_result(ppu: &Ppu) -> String {
    let mut lines = Vec::new();

    // Map is 32x32 tiles, but screen only shows 20x18
    for y in 0..18 {
        let mut line = String::with_capacity(20);
        for x in 0..20 {
            let tile_idx = ppu.read_byte(0x9800 + (y * 32) + x);

            // Map tile index to ASCII (most test ROMs use 1:1 mapping)
            if (32..=126).contains(&tile_idx) {
                line.push(tile_idx as char);
            } else {
                line.push(' ');
            }
        }

        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }

    lines.join("\n")
}
