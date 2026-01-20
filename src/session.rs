use crate::Bus;
use crate::args::Args;
use crate::cartridge::Headers;
use crate::cpu::Cpu;
use crate::testing::doctor_session::DoctorSession;

pub trait SessionHandler {
    fn next(&mut self) -> Result<(), String>;
}

/// Binds together a rom, a register and the flags.
/// Used for holding the entire 'session' of a emulation.
pub struct Session {
    pub memory: Bus,
    pub cpu: Cpu,
    pub headers: Headers,
}

impl Session {
    pub fn new(buffer: Vec<u8>) -> Self {
        let headers = Headers::new(&buffer);
        // info!("Cartridge headers: {:?}", headers);
        Session {
            memory: Bus::new(buffer),
            cpu: Cpu::new(),
            headers,
        }
    }
}
impl SessionHandler for Session {
    fn next(&mut self) -> Result<(), String> {
        self.cpu.step(&mut self.memory);
        return Ok(());
    }
}

pub fn output_string_diff(string_a: &str, string_b: &str) -> String {
    if string_a.len() != string_b.len() {
        panic!(
            "String_diff requires equal lengths. A: {}, B: {}",
            string_a.len(),
            string_b.len()
        );
    }

    // Zip pairs up characters: (a[0], b[0]), (a[1], b[1]), etc.
    string_a
        .chars()
        .zip(string_b.chars())
        .map(|(a, b)| if a == b { ' ' } else { b })
        .collect()
}

pub enum SessionType {
    Normal(Session),
    Doctor(DoctorSession),
}

// Implement the trait for the Enum itself
impl SessionHandler for SessionType {
    fn next(&mut self) -> Result<(), String> {
        match self {
            SessionType::Normal(s) => s.next()?,
            SessionType::Doctor(s) => s.next()?,
        }
        Ok(())
    }
}

pub fn select_session_impl(buffer: Vec<u8>, _args: Args) -> SessionType {
    #[cfg(feature = "doctor")]
    return SessionType::Doctor(DoctorSession::new(buffer, _args));
    #[cfg(not(feature = "doctor"))]
    return SessionType::Normal(Session::new(buffer));
}
