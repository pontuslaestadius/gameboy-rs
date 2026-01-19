use std::path::PathBuf;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of the person to greet
    #[arg(long)]
    pub load_rom: PathBuf,

    // Run the rom in non-interactive "test" mode.
    // Additional operations will be performed.
    // statistics will be gathered during runtime.
    #[arg(long)]
    pub test: bool,

    // Optional log path, if none given, no log will be created.
    #[arg(long)]
    pub log_path: Option<PathBuf>,

    // Run for a predeterminate amount of instructions for Game Boy Doctor emulator test.
    // Provide the number of log lines, or CPU instructions the game expects to verify.
    #[arg(long)]
    pub debug_doctor: Option<usize>,
}
