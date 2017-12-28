#![feature(slice_patterns)]
#![feature(inclusive_range_syntax)]

pub mod instructions;
pub mod share;

/// Decoding reading material:
/// Theory: http://www.z80.info/decoding.htm
/// op code: http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode
use std::io::prelude::*;
use std::fs::File;
use instructions::*;
use share::*;

use std::io;

use std::fs::OpenOptions;

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
    let _ = file.read_to_end(&mut buffer)?;

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
        let opcode: Opcode = session.op_code()?;

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
