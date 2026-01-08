pub mod args;
pub mod binary;
pub mod flags;
pub mod instruction;
pub mod instructions;
pub mod memory;
pub mod registers;
pub mod rom;
pub mod session;
pub mod share;
pub mod utils;

use binary::SmartBinary;
use flags::Flags;
use instruction::Instruction;
use memory::Memory;
use registers::Registers;
use rom::Rom;
use session::Session;
use share::*;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use utils::*;

use std::fs::OpenOptions;

/// Executes the given file and loads it in as a rom.
/// This function is expected to run while the emulation is still going.
pub fn rom_exec(args: args::Args) -> Result<(), io::Error> {
    print_header("LOADING ROM".to_string());
    let mut f = File::open(args.load_rom)?;
    let session = load(&mut f)?;
    let rom_size = session.memory.rom_size;

    print_header(format!("RUNNING ({})", print_size(rom_size)));
    // Starts the main read loop.
    // let invalid = read_loop(session, &args.err_log)?;

    // // Number of valid op codes identified.
    // let valid = rom_size - invalid; // TODO inaccurate, because prefixed and unprefixed OPCODES.
    // let fault_rate = invalid as f64 / ((valid + invalid) as f64) * 100.0;

    // println!("----------- POST-RUN -----------");
    // println!("valid: {}", pretty(valid as f64));
    // println!("invalid: {}", pretty(invalid as f64));
    // println!("fault rate: {}%", pretty(fault_rate));

    Ok(())
}

// Loads in a file as a rom and returns a Session.
fn load(file: &mut File) -> Result<Session, io::Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let _ = file.read_to_end(&mut buffer)?;
    Ok(Session::new(buffer))
}

/// Reads op code forever and is the main loop for the emulation.
/// Will only return anything if it is either done emulating, or
/// if an error occured that made it panic.
fn read_loop(mut session: Session, path: &str) -> Result<usize, io::Error> {
    let mut file = OpenOptions::new().write(true).create(true).open(path)?;

    // Counts the number of invalid op codes read.
    let mut invalid: usize = 0;

    // While pointer counter is on a valid index.
    loop {
        if invalid > 1000 {
            println!("Too many invalid instructions, stopping...");
            break;
        }
        println!("{:?}", session.registers);
        // TODO replace with a permanent loop.
        // let instruction: Instruction = session.fetch_next()?;

        // let result = session.execute(instruction);

        match session.next() {
            Ok(()) => (),
            Err(e) => {
                file.write_all(format!("{:?}", e).as_bytes())?;
                file.write(b"\n")?;
                invalid += 1;
            }
        }
    }

    Ok(invalid)
}
