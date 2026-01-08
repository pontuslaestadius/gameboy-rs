extern crate human_format;

use clap::Parser;
use gameboy_rs;

fn main() -> std::io::Result<()> {
    let args = gameboy_rs::args::Args::parse();
    gameboy_rs::rom_exec(args)
}
