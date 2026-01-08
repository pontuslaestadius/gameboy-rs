use crate::{Opcode, Prefix, SmartBinary};
use std::fmt;

/// Holds a decoded opcode instruction. They can be as either of the following:
/// optional bytes are described using [optional].
/// [prefix byte,]  opcode  [,displacement byte]  [,immediate data]
/// - OR -
/// two prefix bytes,  displacement byte,  opcode
#[derive(PartialEq)]
pub struct Instruction {
    pub raw: SmartBinary,
    pub prefix: Option<Prefix>,
    pub opcode: Opcode,
    pub displacement: Option<i8>,
    pub immediate: (Option<SmartBinary>, Option<SmartBinary>),
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prefix = match &self.prefix {
            Some(x) => format!("Prefix: {:?}, ", x),
            None => format!(""),
        };
        let displacement = match self.displacement {
            Some(x) => format!("displacement: {:?}, ", x),
            None => format!(""),
        };
        write!(
            f,
            "{:?} {:?} code: {:?} {:?} {:?}",
            self.raw, prefix, self.opcode, displacement, self.immediate
        )
    }
}
