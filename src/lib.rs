#![feature(slice_patterns)]
#![feature(inclusive_range_syntax)]

pub mod register;
pub mod instructions;

/// Decoding reading material:
/// Theory: http://www.z80.info/decoding.htm
/// op code: http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode
use std::io::prelude::*;
use std::fs::File;
use std::io::Error;
use instructions::table::*;
use register::*;
use instructions::*;

use std::io;

use std::fs::OpenOptions;

/// -----------------
/// Structures
/// -----------------
///

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct Session {
    rom: Rom,
    registers: Registers,
    flags: Flags,
}

/// Holds an 8-bit binary.
/// Values are stored as booleans because they hold the lowest amount of data in memory.
#[derive(PartialEq)]
pub struct SmartBinary {
    zer: bool,
    one: bool,
    two: bool,
    thr: bool,
    fou: bool,
    fiv: bool,
    six: bool,
    sev: bool,
}

/// Registers are used for virtual emulation storage.
pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

/// Flag documentation gathered from:
/// http://z80.info/z80sflag.htm
/// And has only been stylized but with identical information.
pub struct Flags {
    // (S) -> Set if the 2-complement value is negative (copy of MSB)
    sign: bool,
    // (Z) -> Set if the value is zero
    zero: bool,
    // (F5) -> Copy of bit 5
    five: bool,
    // (H) -> Carry from bit 3 to bit 4
    half_carry: bool,
    // (F3) -> Copy of bit 3
    three: bool,
    // (P/V) ->
    // Parity set if even number of bits set
    // Overflow set if the 2-complement result does not fit in the register
    parity_or_overflow: bool,
    // (N) -> Set if the last operation was a subtraction
    subtract: bool,
    // (C) -> Set if the result did not fit in the register
    carry: bool
}

/// Holds a decoded opcode instruction. They can be as either of the following:
/// optional bytes are described using [optional].
/// [prefix byte,]  opcode  [,displacement byte]  [,immediate data]
/// - OR -
/// two prefix bytes,  displacement byte,  opcode
pub struct Instruction<'a> {
    prefix: Option<Prefix>,
    opcode: Opcode,
    displacement: Option<i8>,
    immediate: (Option<&'a SmartBinary>, Option<&'a SmartBinary>),
}


/// Holds the different types of prefixes that may exists before the opcode.
/// These are hex representations.
/// If the first byte read is any of these, it is always a prefix byte.
pub enum Prefix {
    CB,
    DD,
    ED,
    FD,
}



impl Session {

    /// Steps through to the next instruction to be read and returns the byte.
    pub fn step(&mut self) -> Result<&u8, io::Error> {
        let old_pc = self.registers.get_pc();
        let item = self.rom.get(old_pc)?;
        self.registers.set_pc(old_pc +1);
        Ok(item)
    }

    pub fn step_bytes(&mut self, count: u8) -> Result<Vec<&u8>, io::Error> {
        let mut bytes: Vec<&u8> = Vec::new();

        match count {
            1 => {
                bytes.push(self.step()?);
            }

            _ => { // Assumes 2 // TODO this is so ugly I cry everynight.
                let old_pc = self.registers.get_pc();
                let item1 = self.rom.get(old_pc)?;
                let item2 = self.rom.get(old_pc +1)?;
                bytes.push(item1);
                bytes.push(item2);
                self.registers.set_pc(old_pc +2);

            }

        }

        Ok(bytes)
    }
}


struct Rom {
    content: Vec<u8>,
}


impl Rom {
    pub fn new(content: Vec<u8>) -> Rom {
        Rom {
            content,
        }
    }

    pub fn get(&self, index: u16) -> Result<&u8, io::Error> {
        let item = self.content.get(index as usize)
            .ok_or(io::Error::new(io::ErrorKind::NotFound, "out ot items."))?;
        Ok(item)
    }
}


/// Executes the given file and loads it in as a rom.
/// This function is expected to run while the emulation is still going.
pub fn rom_exec(mut file: &mut File) -> Result<(), io::Error> {

    // Loads the rom in to storage.
    let session = load(&mut file)?;

    // Rom size.
    let rom_size = session.rom.content.len();

    // Starts the main read loop.
    let invalid = read_loop(session)?;

    // Logs all invalid data in a log file.
    log("log.txt", &invalid)?;

    // Number of valid op codes identified.
    let valid = rom_size-invalid.len(); // TODO inaccurate.

    println!("---------- POST-RUN ----------");
    println!("rom size: {}", rom_size);
    println!("valid: {}", valid);
    println!("invalid: {}", invalid.len());
    println!("valid: {}%", (valid as f64/invalid.len() as f64)*100.0);

    Ok(())
}


// Loads in a file as a rom and returns a Session.
fn load(file: &mut File) -> Result<Session, io::Error> {

    let mut buffer: Vec<u8> = Vec::new();
    let rom_size = file.read_to_end(&mut buffer)?;

    // Create the subsystem running the emulation.
    let rom = Rom::new(buffer);
    let registers = Registers::new();
    let flags = Flags::new();

    Ok(
        Session {
        rom,
        registers,
        flags,
    })
}


/// Writes the given vec to the given path.
fn log(path: &str, vec: &Vec<String>) -> Result<(), io::Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;

    for item in vec.iter() {
        file.write_all(item.as_bytes())?;
        file.write(b"\n")?;
    }
    Ok(())
}


/// Reads op code forever and is the main loop for the emulation.
/// Will only return anything if it is either done emulating, or
/// if an error occured that made it panic.
fn read_loop(mut session: Session) -> Result<Vec<String>, io::Error> {

    // Counts the number of invalid op codes read.
    let mut invalid: Vec<String> = Vec::new();

    for _ in 0..32000 { // TODO replace with a permanent loop.
        let (mut opcode, opcodedata): (Opcode, OpCodeData) = session.op_code();

        match opcodedata {
            OpCodeData::BYTE(x) => {
                let mut bytes = session.step_bytes(x)?;
                match opcode {
                    Opcode::JP(_) => opcode = Opcode::JP(bytes_as_octal(bytes)?),
                    _ => panic!("Invalid opcode, fix it ty."),
                }
                ()
            }

            _ => (),
        }

        let formatted_opcode: String = format!("{:?}", opcode); // TODO remove.
        match opcode {

            // Loops for invalid opcodes and stores them in the log file.
            Opcode::INVALID(_) => {
                invalid.push(formatted_opcode);
            }

            _ => println!("{}", formatted_opcode) // TODO replace with execution.
        }
    }

    Ok(invalid)
}
