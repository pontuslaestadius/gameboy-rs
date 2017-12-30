/// Decoding reading material:
/// Theory: http://www.z80.info/decoding.htm
/// op code: http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode
use std::fmt;

use std::io;
use super::share::*;



impl Registers {
    /// Fetches the value given a string from the decode template provided here:
    /// http://www.z80.info/decoding.htm
    pub fn fetch(&self, code: &str) -> u16 {


        // If it's a 8-bit or 16-bit fetch.

        let result = match code.len() {

            // This part is easy.
            1 => {

                match code {
                    "A" => self.a,
                    "B" => self.b,
                    "C" => self.c,
                    "D" => self.d,
                    "E" => self.e,
                    "H" => self.h,
                    "L" => self.l,
                    _ => panic!("Invalid register"),
                }

            }

            _ => panic!("Invalid fetch length"),
        };

        result as u16
    }
}

impl fmt::Debug for SmartBinary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f, "SmartBinary: {:?} -> [{:?}]",
               self.as_list(),
               self.x_y_z_p_q()
        )
    }
}


impl SmartBinary {

    /// Flips the values from 1 to 0 and 0 to 1.
    pub fn flip(&mut self) {
        self.zer = !self.zer;
        self.one = !self.one;
        self.two = !self.two;
        self.thr = !self.thr;
        self.fou = !self.fou;
        self.fiv = !self.fiv;
        self.six = !self.six;
        self.sev = !self.sev;
    }

    pub fn get(&self, bit: u8) -> bool {
        match bit {
            0 => self.zer,
            1 => self.one,
            2 => self.two,
            3 => self.thr,
            4 => self.fou,
            5 => self.fiv,
            6 => self.six,
            7 => self.sev,
            _ => panic!("Invalid bit value: {}", bit)
        }
    }

    pub fn x_y_z_p_q(&self) -> [u8; 5] {

        let orev = |x: bool| {
            if x {
                1
            } else {
                0
            }
        };

        // x = the opcode's 1st octal digit (i.e. bits 7-6)
        let x = octal_digit_from_binary_list(&[
            orev(self.sev),
            orev(self.six)
        ]);

        // y = the opcode's 2nd octal digit (i.e. bits 5-3)
        let y = octal_digit_from_binary_list(&[
            orev(self.fiv),
            orev(self.fou),
            orev(self.thr)
        ]);

        // z = the opcode's 3rd octal digit (i.e. bits 2-0)
        let z = octal_digit_from_binary_list(&[
            orev(self.two),
            orev(self.one),
            orev(self.zer)
        ]);

        // p = y rightshifted one position (i.e. bits 5-4)
        let p = octal_digit_from_binary_list(&[
            orev(self.fiv),
            orev(self.fou),
        ]);

        // q = y modulo 2 (i.e. bit 3)
        let q = octal_digit_from_binary_list(&[
            orev(self.thr),
        ]);

        [x,y,z,p,q]
    }

}

/*
[prefix byte,]  opcode  [,displacement byte]  [,immediate data]
                    - OR -
two prefix bytes,  displacement byte,  opcode
*/


pub fn step_bytes<'a>(rom: &'a Rom, pc: &mut usize, count: u8) -> Result<Vec<&'a u8>, io::Error> {
    let mut bytes: Vec<&u8> = Vec::new();

    for i in 0..count {
        bytes.push(rom.content.get(*pc +i as usize).unwrap());
    }


    *pc = *pc +count as usize;

    Ok(bytes)
}

impl Session {

    pub fn execute(&mut self) -> Result<(), io::Error> {



        Ok(())
    }

    pub fn op_code(&mut self) -> Result<Opcode, io::Error> {

        // Read a single byte from the rom.
        let byte_vec = step_bytes(&self.rom, &mut self.registers.pc, 1).unwrap();

        let byte = byte_vec.get(0).unwrap();
        let binary: SmartBinary = SmartBinary::new(**byte);


        // Check for a prefix byte.
        let (mut opcode, opcodedata) = match check_prefix_opcodes(&binary) {
            None => unprefixed_opcodes(binary),
            Some(Prefix::CB) => unprefixed_opcodes(binary), // TODO replace.
            Some(Prefix::DD) => unprefixed_opcodes(binary), // TODO replace.
            Some(Prefix::ED) => ed_prefixed_opcodes(binary),
            Some(Prefix::FD) => unprefixed_opcodes(binary), // TODO replace.
        };

        match opcodedata {

            OpCodeData::BYTE(x) => {
                let bytes = step_bytes(&self.rom, &mut self.registers.pc, x)?;
                match opcode {
                    // TODO find a better way to do this.
                    Opcode::JP(_) => opcode = Opcode::JP(bytes_as_octal(bytes)?),
                    Opcode::CALL(_) => opcode = Opcode::CALL(bytes_as_octal(bytes)?),
                    Opcode::ALU(y, _) => opcode = Opcode::ALU(y, bytes_as_octal(bytes)? as u8),
                    _ => panic!("Invalid opcode, fix it ty."),
                }
                ()
            }

            OpCodeData::BYTESIGNED(x) => {
                let bytes = step_bytes(&self.rom, &mut self.registers.pc, x)?;
                match opcode {
                    Opcode::JR_(d, _) => opcode = Opcode::JR_(d, bytes_as_octal_signed(bytes)? as i8),
                    _ => panic!("Invalid opcode, fix it ty2."),
                }
            }

            _ => (),
        }

        Ok(opcode)
    }

}


pub fn check_prefix_opcodes(binary: &SmartBinary) -> Option<Prefix> {
    let byte = octal_digit_from_binary_list_u16(&binary.as_list());

    // Check if it matches any of the prefixes in the enum.
    match byte {
        203 => Some(Prefix::CB),
        221 => Some(Prefix::DD),
        237 => Some(Prefix::ED),
        253 => Some(Prefix::FD),
        _ => None,
    }

}


/// This should be called when it is known that it's a unprefixed opcode,
/// Returns a Opcode enum and a number of following bytes required for the action.
pub fn unprefixed_opcodes<'a>(binary: SmartBinary) -> (Opcode, OpCodeData<'a>) {

    // Uses experimental splice patterning.
    let [x,y,z,p,q] = binary.x_y_z_p_q();

    // Used for notifiying caller it needs more data to be executed.
    let mut opcodedata = OpCodeData::NONE;

    // Any commands we can't read, we use an invalid opcode enum.
    let undefined = || {
        Opcode::INVALID(binary)
    };

    let opcode = match x {
        0 => {

            match z {

                0 => {

                    match y {

                        0 => Opcode::NOP,
                        1 => Opcode::EXAF,
                        //2 => Opcode::DJNZ(0), // TODO fix proper value
                        4...7 => {
                            opcodedata = OpCodeData::BYTESIGNED(1);
                            Opcode::JR_(DataTable::CC(y-4), 0)
                        }

                        _ => undefined(),
                    }
                },

                4 => Opcode::INC(y),
                5 => Opcode::DEC(y),

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

        1 => match y {

            6 => Opcode::HALT,

            // TODO should LD always be returned for this?
            _ => Opcode::LD(DataTable::R(y), DataTable::R(z)),
        }

        2 => Opcode::ALU_(y, DataTable::R(z)),

        3 => {

            match z {

                0 => Opcode::RET_(DataTable::CC(y)),

                1 => {

                    match q {

                        1 => {

                            match p {

                                0 => Opcode::RET,
                                1 => Opcode::EXX,
                                2 => Opcode::JPHL, // TODO fix, should be JP(HL)
                                3 => Opcode::LDSPHL,

                                _ => undefined(),

                            }
                        }

                        _ => undefined(),
                    }
                }

                3 =>

                    match y {

                        0 => {
                            opcodedata = OpCodeData::BYTE(2);
                            Opcode::JP(0)
                        },

                        6 => Opcode::DI,
                        7 => Opcode::EI,

                        _ => undefined(),
                    },

                5 => match q {
                    1 => match p {
                        0 => {
                            opcodedata = OpCodeData::BYTE(2);
                            Opcode::CALL(0)
                        }
                        _ => undefined(),
                    }

                     _ => undefined(),
                }

                6 => {
                    opcodedata = OpCodeData::BYTE(1);
                    Opcode::ALU(y,0)
                }

                7 => {
                    Opcode::RST(y*8)
                }

                _ => undefined(),
            }
        }

        _ => undefined(),
    };

    (opcode, opcodedata)
}



/// ED-PREFIXED OPCODES
pub fn ed_prefixed_opcodes<'a>(binary: SmartBinary) -> (Opcode, OpCodeData<'a>) {

    // Uses experimental splice patterning.
    let [x,y,z,p,q] = binary.x_y_z_p_q();

    // Used for notifiying caller it needs more data to be executed.
    let opcodedata = OpCodeData::NONE; // TODO not used for now, thus not mut

    // Any commands we can't read, we use an invalid opcode enum.
    let undefined = || {
        Opcode::INVALID(binary)
    };

    let opcode = match x {
        0 => Opcode::NOP,

        1 => match z {
            4 => Opcode::NEG,
            5 => match y {
                1 => Opcode::RETI,
                _ => Opcode::RETN,
            }

            7 => match y {
                0 => Opcode::LDIA,
                1 => Opcode::LDRA,
                2 => Opcode::LDAI,
                3 => Opcode::LDAR,
                4 => Opcode::RRD,
                5 => Opcode::RLD,
                6...7 => Opcode::NOP,

                _ => undefined(),
            }

            _ => undefined(),
        }
        3 => Opcode::NOP,


        _ => undefined(),
    };

    println!("ED: {:?}", opcode);


    (opcode, opcodedata)
}

pub fn bytes_as_octal_signed(mut vec: Vec<&u8>) -> Result<i16, io::Error> {
    let signed = vec.remove(0);
    let octal = bytes_as_octal(vec)?;
    Ok(
        match *signed {
        0 => octal as i16,
        _ => octal as i16 *-1
    })
}


pub fn bytes_as_octal(vec: Vec<&u8>) -> Result<u16, io::Error> {

    let mut vec_smart_binaries: Vec<SmartBinary> = Vec::new();

    for item in vec.iter() {
        vec_smart_binaries.push(SmartBinary::new(*item.clone()));
    }


    // This part only works for 2 or 1 byte.

    let list1: &SmartBinary = vec_smart_binaries.get(0).unwrap();

    if vec_smart_binaries.len() > 1 {
        // Join the lists.

        // two bytes.
        let mut list: [u8; 16] = [
            0,0,0,0,0,0,0,0,
            0,0,0,0,0,0,0,0];

        let list2: &SmartBinary = vec_smart_binaries.get(1).unwrap();

        let orev = |x: bool| {
            if x {
                1
            } else {
                0
            }
        };

        list[0] = orev(list1.zer);
        list[1] = orev(list1.one);
        list[2] = orev(list1.two);
        list[3] = orev(list1.thr);
        list[4] = orev(list1.fou);
        list[5] = orev(list1.fiv);
        list[6] = orev(list1.six);
        list[7] = orev(list1.sev);

        list[0+8] = orev(list2.zer);
        list[1+8] = orev(list2.one);
        list[2+8] = orev(list2.two);
        list[3+8] = orev(list2.thr);
        list[4+8] = orev(list2.fou);
        list[5+8] = orev(list2.fiv);
        list[6+8] = orev(list2.six);
        list[7+8] = orev(list2.sev);

        Ok(octal_digit_from_binary_list_u16(&list))

    } else {

        Ok(octal_digit_from_binary_list_u16(&list1.as_list()))
    }
}



pub fn octal_digit_from_binary_list(list: &[u8]) -> u8 {
    let mut multiplier = 1;
    let mut result: u8 = 0;

    for item in list.iter().rev() {
        result += item*multiplier;
        multiplier = multiplier*2;
    }
    result
}

pub fn octal_digit_from_binary_list_u16(list: &[u8]) -> u16 {
    let mut multiplier: u32 = 1;
    let mut result: u16 = 0;

    for item in list.iter().rev() {
        result += *item as u16 *multiplier as u16;
        multiplier = multiplier*2;
    }
    result
}

pub fn octal_digit_from_binary_list_i16(list: &[u8]) -> i16 {
    let mut result: i16 = 0;

    let mut iter = list.iter();
    let signed = iter.next().unwrap();

    let signed_clear: i16 = match *signed {
        0 => 1,
        _ => -1
    };

    let two: i16 = 2;
    for (index, item) in iter.rev().enumerate() {
        result += *item as i16 *two.pow(index as u32);
    }
    result*signed_clear
}

