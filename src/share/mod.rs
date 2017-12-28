/// -----------------
/// Structures
/// -----------------

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct Session {
    pub rom: Rom,
    pub registers: Registers,
    pub flags: Flags,
}

/// Holds an 8-bit binary.
/// Values are stored as booleans because they hold the lowest amount of data in memory.
#[derive(PartialEq)]
pub struct SmartBinary {
    pub zer: bool,
    pub one: bool,
    pub two: bool,
    pub thr: bool,
    pub fou: bool,
    pub fiv: bool,
    pub six: bool,
    pub sev: bool,
}

/// Registers are used for virtual emulation storage.
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: Flags,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    // Program counter, used for pointing at the next instruction to be read.
    pub pc: usize,
}

/// Flag documentation gathered from:
/// http://z80.info/z80sflag.htm
/// And has only been stylized but with identical information.
pub struct Flags {
    // (S) -> Set if the 2-complement value is negative (copy of MSB)
    pub sign: bool,
    // (Z) -> Set if the value is zero
    pub zero: bool,
    // (F5) -> Copy of bit 5
    pub five: bool,
    // (H) -> Carry from bit 3 to bit 4
    pub half_carry: bool,
    // (F3) -> Copy of bit 3
    pub three: bool,
    // (P/V) ->
    // Parity set if even number of bits set
    // Overflow set if the 2-complement result does not fit in the register
    pub parity_or_overflow: bool,
    // (N) -> Set if the last operation was a subtraction
    pub subtract: bool,
    // (C) -> Set if the result did not fit in the register
    pub carry: bool
}

/// Holds a decoded opcode instruction. They can be as either of the following:
/// optional bytes are described using [optional].
/// [prefix byte,]  opcode  [,displacement byte]  [,immediate data]
/// - OR -
/// two prefix bytes,  displacement byte,  opcode
pub struct Instruction<'a> {
    pub prefix: Option<Prefix>,
    pub opcode: Opcode,
    pub displacement: Option<i8>,
    pub immediate: (Option<&'a SmartBinary>, Option<&'a SmartBinary>),
}

/// Holds the content of the rom, As to load it in to memory.
pub struct Rom {
    pub content: Vec<u8>,
}


/// -----------------
/// Standardized implementation.
/// -----------------

impl Registers {

    pub fn new() -> Registers {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: Flags::new(),
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
}

impl Flags {

    pub fn new() -> Flags {
        Flags {
            sign: false,
            zero: false,
            five: false,
            half_carry: false,
            three: false,
            parity_or_overflow: false,
            subtract: false,
            carry: false
        }
    }
}

impl Rom {
    pub fn new(content: Vec<u8>) -> Rom {
        Rom {
            content,
        }
    }
}

/// -----------------
/// Enums
/// -----------------

/// Holds the different types of prefixes that may exists before the opcode.
/// These are hex representations.
/// If the first byte read is any of these, it is always a prefix byte.
#[derive(Debug)]
#[derive(PartialEq)]
pub enum Prefix {
    CB,
    DD,
    ED,
    FD,
}

// TODO remove later.
pub enum OpCodeData<'a> {
    REGISTER(&'a str), // Wants data from a register. Str specifies the register.
    BYTE(u8), // Number of follow up bytes to be interpreted as an octal digit.
    NONE, // The opcode has no following data connected to it.
}



/// Holds the data table opcodes use to fetch information.
#[derive(Debug)]
#[derive(PartialEq)]
pub enum DataTable {
    /*
    R(&'a str),
    RP(&'a str),
    RP2(&'a str),
    */
    R(u8),
    CC(u8),
    /*
    ALU(&'a str),
    ROT(&'a str),
    IM(&'a str),
    BLI(&'a str),
    */
}

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

    // z == 4
    INC(u8),    //                  INC r[y]
    // z == 5
    DEC(u8),    //                  DEC r[y]

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
    LD(DataTable, DataTable),   // r[y], r[z]

    // z == 6
    HALT,   // y == 6

    // x == 2
    ALU_(u8, DataTable), // alu[y] r[z]

    // x == 3

    // z == 0
    RET_(DataTable), // RET cc[y]

    // z == 1
    RET,    // p == 0
    EXX,    // p == 1
    JPHL,   // p == 2           JP HL
    LDSPHL, // p == 3           LD SP, HL

    // z == 2
    // z == 3

    JP(u16), // y == 0          JP nn
    DI,     // y == 6
    EI,     // y == 7
    // z == 4
    // z == 5
    // q == 1
    CALL(u16), // p == 0        CALL nn

    ALU(u8, u16), // z == 6 alu[y] n

    RST(u8), // z == 7          RST y*8




    /// ED-PREFIXED OPCODES

    // x == 1
        // z == 4
            NEG,
        // z == 5
            // y == 1
                RETI,
            // y != 1
                RETN,

        // z == 7
            // y == 0
                LDIA, // LD I, A
            // y == 1
                LDRA, // LD R, A
            // y == 2
                LDAI, // LD A, I
            // y == 3
                LDAR, // LD A, R
            // y == 4
                RRD,
            // y == 5
                RLD,


    // If it's an invalid opcode.
    INVALID(SmartBinary)
}