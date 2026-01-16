// use crate::cpu::opcode::opcode_parse_json::load_Json;
use crate::instruction::{CB_OPCODES, Mnemonic, OPCODES};
use crate::{Memory, Registers};
use std::{thread, time};

/// https://8bitnotes.com/2017/05/z80-timing/
const T_CYCLE: time::Duration = time::Duration::from_nanos(250);

// enum CodeResult {
//     Ok,
//     Err(String),
//     NeedsAdditionalBytes(u8),
// }

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct Session {
    pub registers: Registers,
    pub memory: Memory,
    // pub table: Root,
}

impl Session {
    pub fn new(buffer: Vec<u8>) -> Self {
        Session {
            memory: Memory::new(buffer),
            registers: Registers::new(),
            // table: load_Json(),
        }
    }

    // pub fn execute(&mut self, instruction: Instruction) -> Result<(), Instruction> {
    //     let _formatted_opcode: String = format!("{:?}", instruction.opcode); // TODO remove.

    //     match instruction.opcode {
    //         // Loops for invalid opcodes and stores them in the log file.
    //         Opcode::INVALID(_) => {
    //             return Err(instruction);
    //         }

    //         // _ => info!("{}", formatted_opcode) // TODO replace with execution.
    //         _ => (),
    //     }

    //     // TODO
    //     Ok(())
    // }

    pub fn read16(&mut self) -> u16 {
        let p = self.registers.pc as usize;
        self.registers.pc += 2;
        Registers::join(self.memory.data[p], self.memory.data[p + 1])
    }

    pub fn read8(&mut self) -> u8 {
        let p = self.registers.pc as usize;
        self.registers.pc += 1;
        self.memory.data[p]
    }

    pub fn next(&mut self) -> Result<(), String> {
        let byte = self.read8();

        let op = if byte == 0xCB {
            let cb = self.read8();
            CB_OPCODES[cb as usize]
        } else {
            // let hex_key = format!("{:#04X?}", byte);
            // self.table.unprefixed.get(&hex_key)
            OPCODES[byte as usize]
        };
        // Decides if the instruction is implemented, and working as intended.

        if let Some(instruction) = op {
            match instruction.mnemonic {
                Mnemonic::JP => {
                    println!("Code: {:?}", instruction);
                    // We can read here without mutating, as we need to move the PC counter anyways.
                    let p = self.registers.pc as usize;
                    let addr = Registers::join(self.memory.data[p], self.memory.data[p + 1]);
                    self.registers.pc = addr;

                    // So we have 1 byte, the length is 2 bytes more.
                    // So we read 2 bytes as u16, and jump there?
                }
                Mnemonic::NOP => (),
                _ => {
                    panic!(
                        "No handler exists for {:?} | pc: {}",
                        instruction.mnemonic, self.registers.pc
                    );
                }
            }

            for cycle in instruction.cycles {
                thread::sleep(T_CYCLE * *cycle as u32);
            }
        } else {
            panic!("Failed to retrieve opcode for: {:#04X?}", byte);
        }

        // let mut result: Result<(), String> = Ok(());
        return Ok(());

        // match byte {
        //     0 => {
        //         // NOP
        //         // Used for wasting cycles, and thus, waiting.
        //     }
        //     1 => {
        //         // LD BC,d16
        //         let d16 = self.read16();
        //         self.registers.ld_bc(d16);
        //     }
        //     2 => {
        //         // LD (BC),A
        //         self.registers.ld("BC", "A");
        //     }
        //     3 => {
        //         // INC BC
        //         self.registers.inc_bc();
        //     }
        //     4 => {
        //         // INC B
        //         self.registers.inc8('B');
        //     }
        //     5 => {
        //         // DEC B
        //         self.registers.dec8('B');
        //     }
        //     6 => {
        //         // LD B,d8
        //         let d8 = self.read8();
        //         self.registers.ld8('B', d8);
        //     }
        //     7 => {
        //         self.registers.rlca();
        //     }
        //     // 8 => {
        //     //     // LD (a16),SP
        //     //     let data = Registers::join(self.read(1u16), self.read(1u16));
        //     //     self.registers.set_sp(data);
        //     // }
        //     11 => {
        //         // DEC BC
        //         self.registers.dec_bc();
        //     }
        //     12 => {
        //         // INC C
        //         self.registers.inc8('C');
        //     }
        //     13 => {
        //         // DEC C
        //         self.registers.dec8('C');
        //     }
        //     28 => {
        //         // INC E
        //         self.registers.inc8('E');
        //     }
        //     29 => {
        //         // DEC E
        //         self.registers.dec8('E');
        //     }
        //     33 => {
        //         // LD HL,d16
        //         let d16 = self.read16();
        //         self.registers.ld_hc(d16);
        //     }
        //     36 => {
        //         // INC H
        //         self.registers.inc8('H');
        //     }
        //     37 => {
        //         // DEC H
        //         self.registers.dec8('H');
        //     }
        //     44 => {
        //         // INC L
        //         self.registers.inc8('L');
        //     }
        //     45 => {
        //         // DEC L
        //         self.registers.dec8('L');
        //     }
        //     46 => {
        //         // LD L,d8
        //         let d8 = self.read8();
        //         self.registers.ld8('L', d8);
        //     }
        //     60 => {
        //         // INC A
        //         self.registers.inc8('A');
        //     }
        //     61 => {
        //         // INC A
        //         self.registers.dec8('A');
        //     }
        //     64 => {
        //         // LD B,B
        //         self.registers.ld8('B', self.registers.b());
        //     }
        //     65 => {
        //         // LD B,C
        //         self.registers.ld8('B', self.registers.c());
        //     }
        //     66 => {
        //         // LD B,D
        //         self.registers.ld8('B', self.registers.d());
        //     }
        //     67 => {
        //         // LD B,E
        //         self.registers.ld8('B', self.registers.e());
        //     }
        //     68 => {
        //         // LD B,H
        //         self.registers.ld8('B', self.registers.h());
        //     }
        //     69 => {
        //         // LD B,(HL)
        //         self.registers.ld("B", "HL");
        //     }
        //     74 => {
        //         // LD C,D
        //         self.registers.ld8('C', self.registers.d());
        //     }
        //     75 => {
        //         // LD C,E
        //         self.registers.ld8('C', self.registers.e());
        //     }
        //     // 234 => {
        //     //     // LD (a16),A
        //     // }
        //     81 => {
        //         // LD D,C
        //         self.registers.ld("D", "C");
        //     }
        //     82 => {
        //         // LD D,D
        //         self.registers.ld("D", "D");
        //     }
        //     83 => {
        //         // LD D,E
        //         self.registers.ld("D", "E");
        //     }
        //     84 => {
        //         // LD D,H
        //         self.registers.ld("D", "H");
        //     }
        //     85 => {
        //         // LD D,L
        //         self.registers.ld("D", "L");
        //     }
        //     86 => {
        //         // LD D,(HL)
        //         self.registers.ld("D", "HL");
        //     }
        //     87 => {
        //         // LD D,A
        //         self.registers.ld("D", "A");
        //     }
        //     88 => {
        //         // LD E,B
        //         self.registers.ld("E", "B");
        //     }
        //     89 => {
        //         // LD E,C
        //         self.registers.ld("E", "C");
        //     }
        //     90 => {
        //         // LD E,D
        //         self.registers.ld("E", "D");
        //     }
        //     91 => {
        //         // LD E,E
        //         self.registers.ld("E", "E");
        //     }
        //     92 => {
        //         // LD E,H
        //         self.registers.ld("E", "H");
        //     }
        //     93 => {
        //         // LD E,L
        //         self.registers.ld("E", "L");
        //     }
        //     94 => {
        //         // LD E,(HL)
        //         self.registers.ld("E", "HL");
        //     }
        //     95 => {
        //         // LD E,A
        //         self.registers.ld("E", "A");
        //     }
        //     96 => {
        //         // LD H,B
        //         self.registers.ld("H", "B");
        //     }
        //     97 => {
        //         // LD H,C
        //         self.registers.ld("H", "C");
        //     }
        //     98 => {
        //         // LD H,D
        //         self.registers.ld("H", "D");
        //     }
        //     99 => {
        //         // LD H,E
        //         self.registers.ld("H", "E");
        //     }
        //     100 => {
        //         // LD H,H
        //         self.registers.ld("H", "H");
        //     }
        //     101 => {
        //         // LD H,L
        //         self.registers.ld("H", "L");
        //     }
        //     102 => {
        //         // LD H,(HL)
        //         self.registers.ld("H", "HL");
        //     }
        //     103 => {
        //         // LD H,A
        //         self.registers.ld("H", "A");
        //     }
        //     104 => {
        //         // LD L,B
        //         self.registers.ld("L", "B");
        //     }
        //     105 => {
        //         // LD L,C
        //         self.registers.ld("L", "C");
        //     }
        //     106 => {
        //         // LD L,D
        //         self.registers.ld("L", "D");
        //     }
        //     107 => {
        //         // LD L,E
        //         self.registers.ld("L", "E");
        //     }
        //     108 => {
        //         // LD L,H
        //         self.registers.ld("L", "H");
        //     }
        //     109 => {
        //         // LD L,L
        //         self.registers.ld("L", "L");
        //     }
        //     110 => {
        //         // LD L,(HL)
        //         self.registers.ld("L", "HL");
        //     }
        //     111 => {
        //         // LD L,A
        //         self.registers.ld("L", "A");
        //     }
        //     154 => {
        //         self.registers.sub('C');
        //     }
        //     _ => {
        //         result = Err(format!(
        //             "no implementation found for (hex: {:x}, byte: {})",
        //             byte, byte
        //         ));
        //     }
        // }

        // result
    }

    // pub fn fetch_next(&mut self) -> Result<Instruction, io::Error> {
    //     // What I would like to write:
    //     // self.memory.get(self.registers.pc, byte_count)?

    //     // Read a byte from ROM.
    //     let byte = self.read(1u16);
    //     let binary: SmartBinary = SmartBinary::new(byte);

    //     info!("{:x}", byte);

    //     // Check for a prefix byte.
    //     let (prefix, (mut opcode, opcodedata)) = match check_prefix_opcodes(&binary) {
    //         None => (None, unprefixed_opcodes(binary)),
    //         Some(Prefix::CB) => (Some(Prefix::CB), cb_prefixed_opcodes(binary)),
    //         Some(Prefix::DD) => (Some(Prefix::DD), dd_prefixed_opcodes(binary)),
    //         Some(Prefix::ED) => (Some(Prefix::ED), ed_prefixed_opcodes(binary)),
    //         Some(Prefix::FD) => (Some(Prefix::FD), fd_prefixed_opcodes(binary)),
    //     };

    //     match opcodedata {
    //         OpCodeData::BYTE(x) => {
    //             let bytes = step_bytes(&self.memory.data(), &mut self.registers.pc, x)?;
    //             opcode = match opcode {
    //                 // TODO find a better way to do this.
    //                 Opcode::JP(_) => Opcode::JP(bytes_as_octal(bytes)?),
    //                 Opcode::CALL(_) => Opcode::CALL(bytes_as_octal(bytes)?),
    //                 Opcode::CALL_(dt, _) => Opcode::CALL_(dt, bytes_as_octal(bytes)?),
    //                 Opcode::ALU(y, _) => Opcode::ALU(y, bytes_as_octal(bytes)? as u8),
    //                 Opcode::LD_(dt, _) => Opcode::LD_(dt, bytes_as_octal(bytes)?),
    //                 _ => panic!("Invalid opcode for bytes: {:?}", opcode),
    //             };
    //         }

    //         OpCodeData::BYTESIGNED(x) => {
    //             let bytes = step_bytes(&self.memory.data(), &mut self.registers.pc, x)?;
    //             opcode = match opcode {
    //                 Opcode::JR_(d, _) => Opcode::JR_(d, bytes_as_octal_signed(bytes)),
    //                 Opcode::JR(_) => Opcode::JR(bytes_as_octal_signed(bytes)),
    //                 Opcode::DJNZ(_) => Opcode::DJNZ(bytes_as_octal_signed(bytes)),
    //                 _ => panic!("Invalid opcode for bytesigned: {:?}", opcode),
    //             };
    //         }

    //         // It can be no opcodedata, so this is perfectly acceptable.
    //         _ => (),
    //     }

    //     let instruction = Instruction {
    //         raw: SmartBinary::new(**byte),
    //         prefix,
    //         opcode,
    //         displacement: None,
    //         immediate: (None, None),
    //     };

    //     Ok(instruction)
    // }
}
