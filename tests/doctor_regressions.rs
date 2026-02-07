// use std::path::PathBuf;
// use std::process::Command;

mod common;

use std::path::Path;

use gameboy_rs::{mmu::Bus, ppu::Ppu};

use crate::common::{DoctorEvaluator, RuntimeBuilder, RuntimeSession};

// Helper to run the emulator in doctor mode
fn run_doctor_test(rom_id: &str, rom_name: &str) {
    let rom_path = format!(
        "./tests/tools/gb-test-roms/cpu_instrs/individual/{}",
        rom_name
    );
    let golden_log = format!(
        "./tests/tools/gameboy-doctor/truth/unzipped/cpu_instrs/{}.log",
        rom_id
    );

    // Ensure the truth file exists (mimicking your Makefile unzip logic)
    assert!(
        std::path::Path::new(&golden_log).exists(),
        "Truth log missing: {}. Did you unzip them?",
        golden_log
    );

    let mut ppu = Ppu::new();
    ppu.init_post_boot();

    let mut runtime: RuntimeSession<DoctorEvaluator> = RuntimeBuilder::new()
        .with_rom_path(Path::new(&rom_path))
        .with_ppu(ppu)
        .with_evaluator(DoctorEvaluator::new(&golden_log))
        .build();

    runtime.run_to_completition();
}

#[test]
fn doctor_01_special() {
    run_doctor_test("1", "01-special.gb");
}
#[test]
fn doctor_02_interrupts() {
    run_doctor_test("2", "02-interrupts.gb");
}
#[test]
fn doctor_03_sp_hl() {
    run_doctor_test("3", "03-op sp,hl.gb");
}
#[test]
fn doctor_04_r_imm() {
    run_doctor_test("4", "04-op r,imm.gb");
}
#[test]
fn doctor_05_rp() {
    run_doctor_test("5", "05-op rp.gb");
}
#[test]
fn doctor_06_ld_rr() {
    run_doctor_test("6", "06-ld r,r.gb");
}
#[test]
fn doctor_07_jump() {
    run_doctor_test("7", "07-jr,jp,call,ret,rst.gb");
}
#[test]
fn doctor_08_misc() {
    run_doctor_test("8", "08-misc instrs.gb");
}
#[test]
fn doctor_09_op_rr() {
    run_doctor_test("9", "09-op r,r.gb");
}
#[test]
fn doctor_10_bit_ops() {
    run_doctor_test("10", "10-bit ops.gb");
}
#[test]
fn doctor_11_op_a_hl() {
    run_doctor_test("11", "11-op a,(hl).gb");
}
