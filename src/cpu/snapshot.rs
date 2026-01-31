use crate::{constants::*, cpu::Cpu, mmu::Memory};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CpuSnapshot {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub pcmem: [u8; 4], // The 4 bytes at PC
}

#[derive(Debug)]
pub struct StateMismatch {
    pub field: String,
    pub expected: u16, // Use u16 to cover both u8 and u16 registers
    pub actual: u16,
}

impl CpuSnapshot {
    pub fn from_cpu(cpu: &Cpu, bus: &impl Memory) -> Self {
        CpuSnapshot {
            a: cpu.a,
            f: cpu.f,
            b: cpu.b,
            c: cpu.c,
            d: cpu.d,
            e: cpu.e,
            h: cpu.h,
            l: cpu.l,
            sp: cpu.sp,
            pc: cpu.pc,

            // n a very accurate emulator, reading 4 bytes at $PC$ every single step might
            // technically trigger "bus reads" that shouldn't happen (if you have
            // side-effect-heavy hardware mapped to memory). For debugging purposes, this
            // is usually fine, but ensure your bus.read() for the snapshot doesn't
            // accidentally "consume" or "trigger" hardware events (clearing a serial flag).
            pcmem: [
                bus.read_byte(cpu.pc),
                bus.read_byte(cpu.pc.wrapping_add(1)),
                bus.read_byte(cpu.pc.wrapping_add(2)),
                bus.read_byte(cpu.pc.wrapping_add(3)),
            ],
        }
    }

    pub fn pretty_format_flags(&self) -> String {
        let mut string = String::new();
        string.push('[');

        let mut lambda = |flag: u8, letter: char| {
            if self.f & flag != 0 {
                string.push(letter);
            } else {
                string.push('-');
            }
        };

        lambda(FLAG_Z, 'Z');
        lambda(FLAG_N, 'N');
        lambda(FLAG_H, 'H');
        lambda(FLAG_C, 'C');

        string.push(']');
        string
    }
    pub fn to_doctor_string(&self) -> String {
        format!(
            "A:{:02X} F:{} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
            self.a,
            self.pretty_format_flags(),
            self.b,
            self.c,
            self.d,
            self.e,
            self.h,
            self.l,
            self.sp,
            self.pc,
            self.pcmem[0],
            self.pcmem[1],
            self.pcmem[2],
            self.pcmem[3]
        )
    }
    pub fn compare(&self, other: &CpuSnapshot) -> Vec<StateMismatch> {
        let mut diffs = Vec::new();

        if self.a != other.a {
            diffs.push(StateMismatch {
                field: "A".into(),
                expected: self.a as u16,
                actual: other.a as u16,
            });
        }
        if self.pc != other.pc {
            diffs.push(StateMismatch {
                field: "PC".into(),
                expected: self.pc,
                actual: other.pc,
            });
        }
        // ... repeat for other registers ...

        diffs
    }
    pub fn from_string(s: &str) -> Result<Self, String> {
        let mut snapshot = CpuSnapshot::default();
        let parts = s.split_whitespace();

        // Helper to keep the match arms clean
        fn parse_hex8(v: &str) -> Result<u8, String> {
            u8::from_str_radix(v, 16).map_err(|e| e.to_string())
        }

        for part in parts {
            let kv: Vec<&str> = part.split(':').collect();
            if kv.len() != 2 {
                continue;
            }
            let (key, val) = (kv[0], kv[1]);

            match key {
                "A" => snapshot.a = parse_hex8(val)?,
                "F" => snapshot.f = parse_hex8(val)?,
                "B" => snapshot.b = parse_hex8(val)?,
                "C" => snapshot.c = parse_hex8(val)?,
                "D" => snapshot.d = parse_hex8(val)?,
                "E" => snapshot.e = parse_hex8(val)?,
                "H" => snapshot.h = parse_hex8(val)?,
                "L" => snapshot.l = parse_hex8(val)?,
                "SP" => snapshot.sp = u16::from_str_radix(val, 16).map_err(|e| e.to_string())?,
                "PC" => snapshot.pc = u16::from_str_radix(val, 16).map_err(|e| e.to_string())?,
                "PCMEM" => {
                    let bytes: Vec<&str> = val.split(',').collect();
                    for (i, b_str) in bytes.iter().take(4).enumerate() {
                        snapshot.pcmem[i] = parse_hex8(b_str)?;
                    }
                }
                _ => {}
            }
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_state() {
        let line = "A:02 F:50 B:DE C:F4 D:DE E:F5 H:DE L:F6 SP:DFEF PC:C6C6 PCMEM:EA,F6,DE,F1";
        let snap = CpuSnapshot::from_string(line).unwrap();

        assert_eq!(snap.a, 0x02);
        assert_eq!(snap.f, 0x50);
        assert_eq!(snap.sp, 0xDFEF);
        assert_eq!(snap.pc, 0xC6C6);
        assert_eq!(snap.pcmem, [0xEA, 0xF6, 0xDE, 0xF1]);
    }

    #[test]
    fn test_parse_partial_diff() {
        // Simulating a "Was (diff)" line which might only have a few values
        let line = "A:05 PC:0051";
        let snap = CpuSnapshot::from_string(line).unwrap();

        assert_eq!(snap.a, 0x05);
        assert_eq!(snap.pc, 0x0051);
        // Others should be default (0)
        assert_eq!(snap.b, 0x00);
    }
}
