pub mod apu;
pub mod args;
pub mod cartridge;
pub mod constants;
pub mod cpu;
pub mod input;
pub mod mmu;
pub mod opcodes;
pub mod ppu;
pub mod timer;
pub mod utils;

// use crate::cartridge::Headers;
use crate::cpu::Cpu;
use crate::input::RotaryInput;
use crate::ppu::terminal::display_frame;

use constants::*;
use mmu::{Bus, Memory};
use opcodes::*;
use std::io;

use std::path::PathBuf;

use std::io::Write;
use std::time::Instant;

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
            main_loop(buffer);
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
fn main_loop(buffer: Vec<u8>) {
    let mut cpu = Cpu::new();
    // let headers = Headers::new(&buffer);
    let mut bus: Bus<RotaryInput> = Bus::new(buffer);
    let mut last_frame_time = Instant::now();
    loop {
        let mut vblank_triggered = false;
        while !vblank_triggered {
            let cycles = cpu.step(&mut bus);
            if bus.tick_components(cycles) {
                vblank_triggered = true;
            }
        }
        // 2. V-Blank reached! Display the frame
        display_frame(&*bus.ppu);

        // 3. Sleep to maintain original hardware speed
        let elapsed = last_frame_time.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
        last_frame_time = Instant::now();
    }
}
