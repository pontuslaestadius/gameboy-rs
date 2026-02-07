use std::path::PathBuf;

use clap::Parser;
use log::Level;

/// Game Boy Emulator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Log level
    #[arg(long)]
    pub level: Option<Level>,

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
    #[command(flatten)]
    pub doctor: DoctorArgs,
}

#[derive(Parser, Debug, Clone)]
pub struct DoctorArgs {
    #[arg(long)]
    pub golden_log: Option<PathBuf>,
}
