use super::share::*;


struct CPU {
    registers: Registers,

    clock_m: u32,
    clock_t: u32,
    prev_m: u32,
    prev_t: u32,
}