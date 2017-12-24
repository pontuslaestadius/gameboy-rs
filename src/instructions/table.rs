use instructions::Opcode;
use super::super::SmartBinary;


pub fn unprefixed_opcodes<'a>(binary: SmartBinary) -> Opcode {

    // Uses experimental splice patterning.
    let [x,y,z,p,q] = binary.x_y_z_p_q();

    // Any commands we can't read.
    let undefined = || {
        Opcode::INVALID(binary)
    };

    match x {
        0 => {
            match z {
                0 => {
                    match y {
                        0 => Opcode::NOP,
                        1 => Opcode::EXAF,
                        2 => Opcode::DJNZ(0), // TODO

                        _ => undefined(),
                    }
                },

                7 => {
                    match y {
                        0 => Opcode::RLCA,
                        1 => Opcode::RRCA,
                        2 => Opcode::RLA,
                        3 => Opcode::RRA,
                        4 => Opcode::DAA,
                        5 => Opcode::CPL,
                        6 => Opcode::SCF,
                        7 => Opcode::CCF,

                        _ => undefined(),
                    }
                },

                _ => undefined(),
            }
        }

        1 => {
            match y {
                6 => Opcode::HALT,

                _ => undefined(),
            }
        }

        _ => undefined(),
    }
}