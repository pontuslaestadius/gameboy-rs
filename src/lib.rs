pub mod args;
pub mod cartridge;
pub mod constants;
pub mod cpu;
pub mod instruction;
pub mod mmu;
pub mod session;
pub mod testing;
pub mod timer;
pub mod utils;

use crate::session::{SessionHandler, SessionType, select_session_impl};

use constants::*;
use env_logger;
use log::error;
use mmu::memory::Bus;
use mmu::memory_trait;
use std::io;

use std::path::PathBuf;

use std::io::Write;

pub fn setup_logging(log_path: &Option<PathBuf>) -> Result<(), io::Error> {
    let env = env_logger::Env::default().default_filter_or("info");
    let mut builder = env_logger::Builder::from_env(env);

    // 1. Set the format (Crucial for Gameboy Doctor)
    builder.format(|buf, record| writeln!(buf, "{}", record.args()));

    // 2. If a path is provided, redirect output to the file
    if let Some(path) = log_path {
        let file = std::fs::File::create(path)?;
        // We use Target::Pipe to send logs to the file instead of stdout
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }

    builder.init();
    Ok(())
}

/// Executes the given file and loads it in as a rom.
/// This function is expected to run while the emulation is still going.
pub fn rom_exec(args: args::Args) -> Result<(), io::Error> {
    setup_logging(&args.log_path)?;
    match cartridge::load_rom(&args.load_rom) {
        Ok(buffer) => {
            // Starts the main read loop.
            read_loop(select_session_impl(buffer, args))?;
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
fn read_loop(mut session: SessionType) -> Result<(), io::Error> {
    loop {
        match session.next() {
            Ok(()) => (),
            Err(e) => {
                error!("{:?}\n", e);
                panic!("Error");
            }
        }
    }
}
