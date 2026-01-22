#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperandValue {
    U8(u8),
    U16(u16),
    I8(i8),     // Needed for Relative8 (JR offsets)
    Bool(bool), // Needed for Conditions (NZ, Z, etc.)
}

impl OperandValue {
    pub fn as_u8(self) -> u8 {
        match self {
            OperandValue::U8(v) => v,
            _ => panic!("Expected U8, got {:?}", self),
        }
    }

    pub fn as_u16(self) -> u16 {
        match self {
            OperandValue::U16(v) => v,
            OperandValue::U8(v) => v as u16, // Safe promotion
            _ => panic!("Expected U16, got {:?}", self),
        }
    }

    pub fn as_i8(self) -> i8 {
        match self {
            OperandValue::I8(v) => v,
            _ => panic!("Expected I8, got {:?}", self),
        }
    }

    pub fn as_bool(self) -> bool {
        match self {
            OperandValue::Bool(v) => v,
            _ => panic!("Expected Bool, got {:?}", self),
        }
    }
}
