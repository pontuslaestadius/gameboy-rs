#![feature(slice_patterns)]
#![feature(inclusive_range_syntax)]

pub mod instructions;
pub mod share;
pub mod tests;

/// Decoding reading material:
/// Theory: http://www.z80.info/decoding.htm
/// op code: http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode
use std::io::prelude::*;
use std::fs::File;
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
    println!("fault rate: {}%", invalid.len() as f64/((valid+invalid.len()) as f64)*100.0);

    Ok(())
}


// Loads in a file as a rom and returns a Session.
fn load(file: &mut File) -> Result<Session, io::Error> {

    let mut buffer: Vec<u8> = Vec::new();
    let _ = file.read_to_end(&mut buffer)?;

    // Create the subsystem running the emulation.
    let rom = Rom::new(buffer);
    let registers = Registers::new();

    Ok(
        Session {
        rom,
        registers,
    })
}


/// Writes the given vec to the given path.
fn log(path: &str, vec: &Vec<Instruction>) -> Result<(), io::Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;

    for item in vec.iter() {
        file.write_all(format!("{:?}", item).as_bytes())?;
        file.write(b"\n")?;
    }
    Ok(())
}


/// Reads op code forever and is the main loop for the emulation.
/// Will only return anything if it is either done emulating, or
/// if an error occured that made it panic.
fn read_loop(mut session: Session) -> Result<Vec<Instruction>, io::Error> {

    // Counts the number of invalid op codes read.
    let mut invalid: Vec<Instruction> = Vec::new();

    while session.registers.pc < session.rom.content.len() { // TODO replace with a permanent loop.
        let instruction: Instruction = session.fetch_next()?;

        let result = session.execute(instruction);

        match result {
            Ok(()) => (),
            Err(e) => invalid.push(e)
        }
    }

    Ok(invalid)
}
