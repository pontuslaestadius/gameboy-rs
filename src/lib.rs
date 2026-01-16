pub mod args;
pub mod binary;
pub mod cpu;
pub mod instruction;
pub mod instructions;
pub mod memory;
pub mod registers;
pub mod rom;
pub mod session;
pub mod share;
pub mod utils;

use binary::SmartBinary;
use instruction::Instruction;
use log::info;
use memory::Memory;
use registers::Registers;
use session::Session;
use share::*;
use std::io;
use utils::*;

use std::fs::OpenOptions;

use log::LevelFilter;

/// Executes the given file and loads it in as a rom.
/// This function is expected to run while the emulation is still going.
pub fn rom_exec(args: args::Args) -> Result<(), io::Error> {
    if let Some(log_path) = args.log_path {
        OpenOptions::new().write(true).create(true);
        simple_logging::log_to_file(log_path, LevelFilter::Info)?;
    }

    match rom::load_rom(&args.load_rom) {
        Ok(session) => {
            let rom_size = session.memory.rom_size;

            print_header(format!("RUNNING ({})", print_size(rom_size)));
            // Starts the main read loop.
            let invalid = read_loop(session)?;

            if args.test {
                // // Number of valid op codes identified.
                let valid = rom_size - invalid; // TODO inaccurate, because prefixed and unprefixed OPCODES.
                let fault_rate = invalid as f64 / ((valid + invalid) as f64) * 100.0;
                info!("----------- POST-RUN -----------");
                info!("valid: {}", pretty(valid as f64));
                info!("invalid: {}", pretty(invalid as f64));
                info!("fault rate: {}%", pretty(fault_rate));
            }
        }
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }

    Ok(())
}

/// Reads op code forever and is the main loop for the emulation.
/// Will only return anything if it is either done emulating, or
/// if an error occured that made it panic.
fn read_loop(mut session: Session) -> Result<usize, io::Error> {
    // Counts the number of invalid op codes read.
    let mut invalid: usize = 0;

    // While pointer counter is on a valid index.
    loop {
        if invalid > 1000 {
            info!("Too many invalid instructions, stopping...");
            break;
        }
        info!("{:?}", session.registers);
        // TODO replace with a permanent loop.
        // let instruction: Instruction = session.fetch_next()?;

        // let result = session.execute(instruction);

        match session.next() {
            Ok(()) => (),
            Err(e) => {
                info!("{:?}\n", e);
                invalid += 1;
            }
        }
    }

    Ok(invalid)
}
