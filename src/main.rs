extern crate paw;
extern crate human_format;

use gameboy_rs;

#[paw::main]
fn main(args: gameboy_rs::Args) -> std::io::Result<()> {
    gameboy_rs::rom_exec(args)
}
