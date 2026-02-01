use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use std::collections::VecDeque;
use std::sync::Mutex;

// This is merely a limit to keep the log from being too crowded,
// but at the same time, if we underlog, we need to re-test.
const RING_BUFFER_SIZE: usize = 60;

lazy_static::lazy_static! {
    static ref LOG_BUFFER: Mutex<VecDeque<String>> = Mutex::new(VecDeque::with_capacity(RING_BUFFER_SIZE));
}

struct RingLogger;

impl log::Log for RingLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut buffer = LOG_BUFFER.lock().unwrap();
            if buffer.len() >= RING_BUFFER_SIZE {
                buffer.pop_front();
            }
            // buffer.push_back(format!("{}: {}", record.level(), record.args()));
            buffer.push_back(format!("{}", record.args()));
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() -> Result<(), SetLoggerError> {
    log::set_logger(&RingLogger).map(|()| log::set_max_level(LevelFilter::Trace))
}

pub fn dump_log() {
    let buffer = LOG_BUFFER.lock().unwrap();
    let len = buffer.len();
    if len == 0 {
        println!("Log buffer is empty.");
        return;
    }

    let half = (len + 1) / 2;
    // Col width doesn't include the full line, so if we want 90 total,
    // we need to estimate ~13 chars extra, so total 77 / 2 = ~38.
    let col_width = 40; // Adjust as needed for your terminal

    println!(
        "--- LAST {} LOG LINES (Relative Index | Two-Column) ---",
        len
    );

    for i in 0..half {
        // Calculate relative indices (e.g., -100, -99... 0)
        let left_idx = (i as i32) - (len as i32) + 1;
        let left_val = truncate(&buffer[i], col_width);

        if i + half < len {
            let right_idx = ((i + half) as i32) - (len as i32) + 1;
            let right_val = truncate(&buffer[i + half], col_width);

            println!(
                // Assumes it's always less than 3 character, e.g. <100.
                "{:>3}. {:<width$} | {:>3}. {}",
                left_idx,
                left_val,
                right_idx,
                right_val,
                width = col_width
            );
        } else {
            println!("{:>3}. {}", left_idx, left_val);
        }
    }
}

fn truncate(s: &str, max_width: usize) -> String {
    if s.len() > max_width {
        let mut truncated = s[..max_width - 3].to_string();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
    }
}
