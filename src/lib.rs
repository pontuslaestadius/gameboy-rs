pub mod args;
pub mod cartridge;
pub mod constants;
pub mod cpu;
pub mod instruction;
pub mod memory;
pub mod memory_trait;
pub mod registers;
pub mod session;
pub mod utils;

use constants::*;
use env_logger;
use log::{error, info};
use memory::Memory;
use session::Session;
use std::io;

use std::fs::OpenOptions;
use std::path::PathBuf;

use log::LevelFilter;

pub fn setup_logging(log_path: Option<PathBuf>) -> Result<(), io::Error> {
    if let Some(log_path) = log_path {
        OpenOptions::new().write(true).create(true);
        simple_logging::log_to_file(log_path, LevelFilter::Info)?;
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp(None)
            .init();
    }
    Ok(())
}

/// Executes the given file and loads it in as a rom.
/// This function is expected to run while the emulation is still going.
pub fn rom_exec(args: args::Args) -> Result<(), io::Error> {
    setup_logging(args.log_path);
    match cartridge::load_rom(&args.load_rom) {
        Ok(session) => {
            // Starts the main read loop.
            read_loop(session)?;
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
fn read_loop(mut session: Session) -> Result<(), io::Error> {
    // While pointer counter is on a valid index.
    loop {
        // TODO replace with a permanent loop.

        match session.next() {
            Ok(()) => (),
            Err(e) => {
                error!("{:?}\n", e);
                panic!("Error");
            }
        }
    }

    Ok(())
}
