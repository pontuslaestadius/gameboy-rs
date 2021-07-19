pub mod binary;
pub mod flags;
pub mod instructions;
pub mod registers;
pub mod rom;
pub mod share;
pub mod session;

use flags::Flags;
use registers::Registers;
use rom::Rom;
use session::Session;
use binary::SmartBinary;
use share::*;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use human_format::{Formatter, Scales};
use std::fs::OpenOptions;

// With the "paw" feature enabled in structopt
#[derive(structopt::StructOpt)]
pub struct Args {
    /// GBN file to execute
    #[structopt(short = "g", long = "gba")]
    pub gba: String,

    /// Error Log path
    #[structopt(long = "log_path", default_value = "error.log")]
    pub err_log: String,
}

/// Executes the given file and loads it in as a rom.
/// This function is expected to run while the emulation is still going.
pub fn rom_exec(args: Args) -> Result<(), io::Error> {
    println!("---------- LOADING ROM ----------");
    let mut f = File::open(args.gba)?;
    let session = load(&mut f)?;
    println!("{:?}", session.registers);
    let rom_size = session.rom.content.len();

    print_size(rom_size);
    println!("------------ RUNNING ------------");
    // Starts the main read loop.
    let invalid = read_loop(session, &args.err_log)?;

    // Number of valid op codes identified.
    let valid = rom_size - invalid; // TODO inaccurate, because prefixed and unprefixed OPCODES.
    let fault_rate = invalid as f64 / ((valid + invalid) as f64) * 100.0;

    println!("----------- POST-RUN -----------");
    println!("valid: {}", pretty(valid as f64));
    println!("invalid: {}", pretty(invalid as f64));
    println!("fault rate: {}%", pretty(fault_rate));

    Ok(())
}

fn pretty<T: Into<f64>>(size: T) -> String {
    Formatter::new().with_decimals(2).format(size.into())
}

/// Pretty-formatting size.
fn print_size(size: usize) {
    let mut scales = Scales::new();
    scales.with_base(1024).with_suffixes(vec!["B", "kB", "MB"]);
    let result = Formatter::new().with_scales(scales).format(size as f64);
    println!("size: {}", result);
}

// Loads in a file as a rom and returns a Session.
fn load(file: &mut File) -> Result<Session, io::Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let _ = file.read_to_end(&mut buffer)?;

    // Create the subsystem running the emulation.
    let rom = Rom::new(buffer);
    let registers = Registers::new();
    let flags = Flags::new();

    Ok(Session {
        rom,
        registers,
    })
}

// /// Writes the given vec to the given path.
// fn log(path: &str, vec: &Vec<Instruction>) -> Result<(), io::Error> {
//     let mut file = OpenOptions::new().write(true).create(true).open(path)?;

//     for item in vec.iter() {
//         file.write_all(format!("{:?}", item).as_bytes())?;
//         file.write(b"\n")?;
//     }
//     Ok(())
// }

/// Reads op code forever and is the main loop for the emulation.
/// Will only return anything if it is either done emulating, or
/// if an error occured that made it panic.
fn read_loop(mut session: Session, path: &str) -> Result<usize, io::Error> {
    let mut file = OpenOptions::new().write(true).create(true).open(path)?;

    // Counts the number of invalid op codes read.
    let mut invalid: usize = 0;

    let len = session.rom.content.len();

    // While pointer counter is on a valid index.
    while (session.registers.pc as usize) < len {
        // TODO replace with a permanent loop.
        let instruction: Instruction = session.fetch_next()?;

        let result = session.execute(instruction);

        match result {
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
