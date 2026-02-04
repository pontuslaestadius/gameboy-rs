// use std::path::PathBuf;
// use std::process::Command;

mod common;

use std::path::Path;

use crate::common::{RuntimeBuilder, RuntimeSession, serial_evaluator::SerialEvaluator};

// Helper to run the emulator in doctor mode
fn run_test(rom_path: &str) {
    let mut runtime: RuntimeSession<SerialEvaluator> = RuntimeBuilder::new()
        .with_rom_path(Path::new(&rom_path))
        .with_evaluator(SerialEvaluator::new())
        .build();

    runtime.run_to_completition();
}

#[test]
fn mem_timing() {
    run_test("tests/tools/gb-test-roms/cpu_instrs/individual/01-special.gb");
}
