use crate::instructions::*;
use crate::share::*;
use crate::{Instruction, Registers, Rom, SmartBinary};
use std::io;

/// 16 MB (1024*1024*16)
const ADDRESS_BOOK_SIZE: usize = 16777216;

struct AdressBook {
    // 32MB max limit.
    data: [u16; ADDRESS_BOOK_SIZE],
}

impl AdressBook {
    pub fn new() -> AdressBook {
        AdressBook { data: [0; ADDRESS_BOOK_SIZE] }
    }
}

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct Session {
    pub rom: Rom,
    pub registers: Registers,
}

impl Session {
    pub fn execute(&mut self, instruction: Instruction) -> Result<(), Instruction> {
        let _formatted_opcode: String = format!("{:?}", instruction.opcode); // TODO remove.

        match instruction.opcode {
            // Loops for invalid opcodes and stores them in the log file.
            Opcode::INVALID(_) => {
                return Err(instruction);
            }

            // _ => println!("{}", formatted_opcode) // TODO replace with execution.
            _ => (),
        }

        // TODO
        Ok(())
    }

    pub fn fetch_next(&mut self) -> Result<Instruction, io::Error> {
        // Read a single byte from the rom.
        let byte_vec = step_bytes(&self.rom, &mut self.registers.pc, 1).unwrap();

        let byte = byte_vec.get(0).unwrap();
        let binary: SmartBinary = SmartBinary::new(**byte);

        // Check for a prefix byte.
        let (prefix, (mut opcode, opcodedata)) = match check_prefix_opcodes(&binary) {
            None => (None, unprefixed_opcodes(binary)),
            Some(Prefix::CB) => (Some(Prefix::CB), cb_prefixed_opcodes(binary)),
            Some(Prefix::DD) => (Some(Prefix::DD), dd_prefixed_opcodes(binary)),
            Some(Prefix::ED) => (Some(Prefix::ED), ed_prefixed_opcodes(binary)),
            Some(Prefix::FD) => (Some(Prefix::FD), fd_prefixed_opcodes(binary)),
        };

        match opcodedata {
            OpCodeData::BYTE(x) => {
                let bytes = step_bytes(&self.rom, &mut self.registers.pc, x)?;
                opcode = match opcode {
                    // TODO find a better way to do this.
                    Opcode::JP(_) => Opcode::JP(bytes_as_octal(bytes)?),
                    Opcode::CALL(_) => Opcode::CALL(bytes_as_octal(bytes)?),
                    Opcode::CALL_(dt, _) => Opcode::CALL_(dt, bytes_as_octal(bytes)?),
                    Opcode::ALU(y, _) => Opcode::ALU(y, bytes_as_octal(bytes)? as u8),
                    Opcode::LD_(dt, _) => Opcode::LD_(dt, bytes_as_octal(bytes)?),
                    _ => panic!("Invalid opcode for bytes: {:?}", opcode),
                };
            }

            OpCodeData::BYTESIGNED(x) => {
                let bytes = step_bytes(&self.rom, &mut self.registers.pc, x)?;
                opcode = match opcode {
                    Opcode::JR_(d, _) => Opcode::JR_(d, bytes_as_octal_signed(bytes)),
                    Opcode::JR(_) => Opcode::JR(bytes_as_octal_signed(bytes)),
                    Opcode::DJNZ(_) => Opcode::DJNZ(bytes_as_octal_signed(bytes)),
                    _ => panic!("Invalid opcode for bytesigned: {:?}", opcode),
                };
            }

            // It can be no opcodedata, so this is perfectly acceptable.
            _ => (),
        }

        let instruction = Instruction {
            raw: SmartBinary::new(**byte),
            prefix,
            opcode,
            displacement: None,
            immediate: (None, None),
        };

        Ok(instruction)
    }
}
