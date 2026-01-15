extern crate human_format;

use clap::Parser;

fn main() -> std::io::Result<()> {
    let args = gameboy_rs::args::Args::parse();
    gameboy_rs::rom_exec(args)
}
