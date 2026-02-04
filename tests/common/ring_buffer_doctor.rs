use gameboy_rs::{cpu::CpuSnapshot, opcodes::OpcodeInfo};

const RING_BUFFER_LENGTH: usize = 5;

pub struct RingBufferDoctor {
    pub entries: [Option<RingBufferDoctorState>; RING_BUFFER_LENGTH],
    pub head: usize,
}

impl Default for RingBufferDoctor {
    fn default() -> Self {
        Self::new()
    }
}

impl RingBufferDoctor {
    pub fn new() -> Self {
        // Initialize with None because the buffer is empty at start
        Self {
            entries: Default::default(),
            head: 0,
        }
    }

    pub fn last(&self) -> Option<RingBufferDoctorState> {
        let i = if self.head == 0 {
            RING_BUFFER_LENGTH - 1
        } else {
            self.head - 1
        };
        self.entries[i].clone()
    }

    pub fn push(&mut self, instruction: OpcodeInfo, state: CpuSnapshot, line: usize) {
        self.entries[self.head] = Some(RingBufferDoctorState {
            instruction,
            state,
            line,
        });
        // Wrap around using the modulo operator
        self.head = (self.head + 1) % RING_BUFFER_LENGTH;
    }

    /// Returns the history from oldest to newest
    pub fn get_history(&self) -> Vec<&RingBufferDoctorState> {
        let mut history = Vec::new();
        for i in 0..RING_BUFFER_LENGTH {
            // Start from head (oldest) and go around
            let idx = (self.head + i) % RING_BUFFER_LENGTH;
            if let Some(ref entry) = self.entries[idx] {
                history.push(entry);
            }
        }
        history
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RingBufferDoctorState {
    pub instruction: OpcodeInfo,
    pub state: CpuSnapshot,
    pub line: usize,
}
