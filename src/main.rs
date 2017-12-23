
extern crate gameboyrs;

use std::fs::File;

fn main() {

    // Creates the registers.
    let registers = gameboyrs::register::Registers::new();

    let filename = "/home/pontus/Desktop/Tetris (World).gb";

    let mut f = File::open(filename).expect("file not found");

    gameboyrs::rom_exec(&mut f);
}
