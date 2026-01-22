use super::*;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct FlagSpec {
    pub z: FlagAction,
    pub n: FlagAction,
    pub h: FlagAction,
    pub c: FlagAction,
}

impl fmt::Display for FlagSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We use the standard ZNHC order
        // Using 'v' for Calculate, but you can override it for specific positions
        let z = if self.z == FlagAction::Calculate {
            'Z'
        } else {
            format!("{}", self.z).chars().next().unwrap()
        };
        let n = if self.n == FlagAction::Calculate {
            'N'
        } else {
            format!("{}", self.n).chars().next().unwrap()
        };
        let h = if self.h == FlagAction::Calculate {
            'H'
        } else {
            format!("{}", self.h).chars().next().unwrap()
        };
        let c = if self.c == FlagAction::Calculate {
            'C'
        } else {
            format!("{}", self.c).chars().next().unwrap()
        };

        write!(f, "[{}{}{}{}]", z, n, h, c)
    }
}
