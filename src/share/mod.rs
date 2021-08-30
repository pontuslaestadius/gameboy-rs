use crate::binary::*;



/// -----------------
/// Enums
/// -----------------

/// Holds the different types of prefixes that may exists before the opcode.
/// These are hex representations.
/// If the first byte read is any of these, it is always a prefix byte.
#[derive(Debug, PartialEq)]
pub enum Prefix {
    CB,
    DD,
    ED,
    FD,
}

// TODO remove later.
#[derive(Debug, PartialEq)]
pub enum OpCodeData<'a> {
    REGISTER(&'a str), // Wants data from a register. Str specifies the register.
    BYTE(u8),          // Number of follow up bytes to be interpreted as an octal digit.
    BYTESIGNED(u8),    // Same as BYTE but will return a signed version.
    NONE,              // The opcode has no following data connected to it.
}

/// Holds the data table opcodes use to fetch information.
/// These are intended to point towards a point of data, and does not store the data.
#[derive(Debug, PartialEq)]
pub enum DataTable {
    RP(u8),
    RP2(u8),
    R(u8),
    CC(u8),
    ALU(u8),
    ROT(u8),
    IM(u8),
    BLI(u8),
}

/// http://www.z80.info/decoding.htm
#[derive(Debug, PartialEq)]
pub enum Opcode {
    // unprefixed opcodes

    // x == 0
    // z == 0
    NOP,                // y == 0           NOP
    EXAF,               // y == 1           EX AF, AF'
    DJNZ(i8),           // y == 2           DJNZ d
    JR(i8),             // y == 3           JR d
    JR_(DataTable, i8), // 4 => y <= 7      JR cc[y-4], d

    // z == 1
    LD_(DataTable, u16), // q == 1  LD rp[p], nn
    // q == 2           ADD HL, rp[p]

    // z == 4
    INC(u8), //                  INC r[y]
    // z == 5
    DEC(u8), //                  DEC r[y]

    // z == 7
    RLCA, // y == 0
    RRCA, // y == 1
    RLA,  // y == 2
    RRA,  // y == 3
    DAA,  // y == 4
    CPL,  // y == 5
    SCF,  // y == 6
    CCF,  // y == 7

    // x == 1
    LD(DataTable, DataTable), // r[y], r[z]

    // z == 6
    HALT, // y == 6

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
    DI,      // y == 6
    EI,      // y == 7
    // z == 4
    CALL_(DataTable, u16), //   CALL cc[y], nn

    // z == 5
    // q == 1
    CALL(u16), // p == 0        CALL nn

    ALU(u8, u8), // z == 6 alu[y] n

    RST(u8), // z == 7          RST y*8

    /// PREFIXED OPCODES
    CB(CB),
    ED(ED),
    DD(DD),
    FD(FD),

    // If it's an invalid opcode.
    INVALID(SmartBinary),
}

/// Holds the ED-prefixed opcodes.
#[derive(Debug, PartialEq)]
pub enum ED {
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

    SBCHL(u16), //SBC HL, rp[p]
    ADCHL(u16), //ADC HL, rp[p]

    // x == 2
    BLI(u8, u8), // bli[y,z]
}

/// Holds the CB-prefixed opcodes.
#[derive(Debug, PartialEq)]
pub enum CB {
    ROT(u8, DataTable), // rot[y] r[z]
    BIT(u8, DataTable), // BIT y, r[z]
    RES(u8, DataTable), // RES y, r[z]
    SET(u8, DataTable), // SET y, r[z]
}

/// Holds the FD-prefixed opcodes.
#[derive(Debug, PartialEq)]
pub enum FD {}

/// Holds the DD-prefixed opcodes.
#[derive(Debug, PartialEq)]
pub enum DD {}
