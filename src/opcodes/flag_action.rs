use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FlagAction {
    None,      // "-" (Not affected)
    Set,       // "1" (Always set)
    Reset,     // "0" (Always reset)
    Calculate, // "Z", "N", "H", or "C" (Computed at runtime)
    Invert,    // Added for CCF (Complement Carry Flag)
}

impl fmt::Display for FlagAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let c = match self {
            FlagAction::None => '-',      // Not affected
            FlagAction::Calculate => 'v', // Varies/Calculated (or use 'Z','N', etc)
            FlagAction::Set => '1',       // Hardcoded Set
            FlagAction::Reset => '0',     // Hardcoded Reset
            FlagAction::Invert => '!',
        };
        write!(f, "{}", c)
    }
}
