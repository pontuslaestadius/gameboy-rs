pub mod table;
use super::SmartBinary;


/// http://www.z80.info/decoding.htm
#[derive(Debug)]
#[derive(PartialEq)]
pub enum Opcode {

    // unprefixed opcodes

    // x == 0
        // z == 0
        NOP,        // y == 0           NOP
        EXAF,       // y == 1           EX AF, AF'
        DJNZ(i8),   // y == 2           DJNZ d
        JR(i8),     // y == 3           JR d
        JRCC(i8),   // 4 => y <= 7      JR cc[y-4], d // TODO wtf is this?

    // z == 1
        // q == 1           LD rp[p], nn
        // q == 2           ADD HL, rp[p]







        // z == 7
            RLCA,   // y == 0
            RRCA,   // y == 1
            RLA,    // y == 2
            RRA,    // y == 3
            DAA,    // y == 4
            CPL,    // y == 5
            SCF,    // y == 6
            CCF,    // y == 7


    // x == 1
        // z == 6
            HALT,   // y == 6

    // x == 3
        // z == 0
        // z == 1
            RET,    // p == 0
            EXX,    // p == 1
            JPHL,   // p == 2           JP HL
            LDSPHL, // p == 3           LD SP, HL

        // z == 2
        // z == 3

            JP(u16), // y == 0          JP nn
        // z == 4
        // z == 5
        // z == 6
        // z == 7
            RST(u8), //                 RST y*8


    // If it's an invalid opcode.
    INVALID(SmartBinary)
}
