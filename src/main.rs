
extern crate gameboyrs;

use std::fs::File;

fn main() {

    // Rom to load.
    let filename = "/home/pontus/Desktop/Tetris (World).gb";

    // Open as a file.
    let mut f = File::open(filename).expect("file not found");

    // Execute the emulation with the file.
    let _ = gameboyrs::rom_exec(&mut f);
}
