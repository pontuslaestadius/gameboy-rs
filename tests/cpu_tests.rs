use gameboy_rs::constants::{
    ADDR_SYS_IE, ADDR_SYS_IF, ADDR_TIMER_TAC, ADDR_TIMER_TIMA, ADDR_TIMER_TMA, ADDR_VEC_VBLANK,
};
use gameboy_rs::cpu::{Cpu, StepFlowController};
use gameboy_rs::input::DummyInput;
use gameboy_rs::mmu::Bus;
use gameboy_rs::mmu::Memory;
use gameboy_rs::opcodes::{InstructionSet, Mnemonic, OPCODES};
use gameboy_rs::ppu::Ppu;

const NOP: u8 = 0x00;
const HALT: u8 = 0x76;
const INC_A: u8 = 0x3C;

fn bootstrap() -> (Cpu, Bus<DummyInput>) {
    // RUST_LOG=trace cargo test cpu::test::test_ei_delay_timing -- --nocapture
    // let _ = env_logger::builder().is_test(true).try_init();
    let bus: Bus<DummyInput> = Bus::new(Vec::new()); // Your memory/system component
    let cpu = Cpu::new();
    (cpu, bus)
}

#[test]
fn test_ei_delay_timing() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Execute EI (Opcode 0xFB)
    cpu.pc = 0x100;
    bus.force_write_byte(0x100, 0xFB); // EI
    cpu.step(&mut bus);

    // After EI, IME should still be false, but scheduled
    assert!(!cpu.ime, "IME should not be enabled immediately after EI");
    assert_eq!(
        cpu.ime_scheduled, 1,
        "IME should be scheduled for next step"
    );

    // 2. Execute a NOP (Opcode 0x00)
    bus.write_byte(0x101, 0x00);
    cpu.step(&mut bus);

    // After the instruction FOLLOWING EI, IME becomes true
    assert!(
        cpu.ime,
        "IME should be enabled after the instruction following EI"
    );
    assert_eq!(cpu.ime_scheduled, 0);
}

#[test]
fn test_ei_timing_strict() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Execute EI
    cpu.pc = 0x100;
    bus.force_write_byte(0x100, 0xFB); // EI
    cpu.step(&mut bus);

    // After EI finishes, the 'delay' should be primed
    // If your step decrements BEFORE fetch, this should be 1.
    assert_eq!(
        cpu.ime_scheduled, 1,
        "IME should be 1 step away from enabling"
    );
    assert!(!cpu.ime, "IME should still be false");

    // 2. Execute any other instruction (e.g., NOP)
    bus.write_byte(0x101, 0x00);
    cpu.step(&mut bus);

    // Now IME must be true
    assert!(
        cpu.ime,
        "IME should have enabled after this instruction finished"
    );
}

#[test]
fn test_interrupt_trigger_timing_sequence() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Initialize to a known clean state
    cpu.pc = 0x100;
    cpu.sp = 0xDFFD;
    cpu.ime = false;
    cpu.ime_scheduled = 0;
    cpu.a = 0x01; // Value to be written to IF

    // Setup: Enable V-Blank in IE (0xFFFF)
    bus.write_ie(0x01);

    // --- STEP 1: EI (FB) ---
    bus.force_write_bytes(cpu.pc, &[0xFB, 0xE0, 0x0F]);
    cpu.step(&mut bus);

    assert_eq!(cpu.pc, 0x101, "PC should move to next instr");
    assert!(!cpu.ime, "IME should not be active yet");
    assert_eq!(cpu.ime_scheduled, 1, "IME should be scheduled");

    // --- STEP 2: LDH (0xFF0F), A (E0 0F) ---
    // This instruction enables the interrupt flag.
    cpu.step(&mut bus);

    assert_eq!(cpu.pc, 0x103, "PC should move past LDH");
    assert!(
        cpu.ime,
        "IME should enable AFTER the instruction following EI"
    );
    assert_eq!(bus.read_byte(0xFF0F), 0x01, "IF should now be set");

    // --- STEP 3: THE INTERRUPT HIJACK ---
    // The CPU is at 0x103. IME is true. IF is 0x01.
    // In a real Game Boy, the interrupt is serviced BEFORE 0x103 executes.
    bus.write_byte(0x103, 0x00); // NOP (should be 'skipped' or 'delayed')
    cpu.step(&mut bus);

    // Assertions for a successful Hijack
    assert_eq!(cpu.pc, 0x0040, "PC should be at the V-Blank vector");
    assert_eq!(cpu.sp, 0xDFFB, "SP should have decreased by 2");

    // Verify what was pushed to the stack
    assert_eq!(
        bus.read_u16(cpu.sp),
        0x103,
        "Stack must save the address of the instruction we jumped over"
    );
}

#[test]
fn test_halt_bug_trigger() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.ime = false;
    bus.write_ie(0x01); // IE: Enable V-Blank
    bus.write_if(0x01); // IF: Request V-Blank (Already pending!)

    // Execute HALT (Opcode HALT)
    bus.force_write_byte(0x100, HALT);
    cpu.step(&mut bus);

    assert!(
        cpu.halt_bug_triggered,
        "Halt bug should trigger when IME is off and interrupt is pending"
    );
    assert!(
        !cpu.halted,
        "CPU should NOT enter halt state when halt bug triggers"
    );
}
#[test]
fn test_halt_bug_execution_cycle() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Setup Halt Bug conditions: IME off, but Interrupt Pending
    cpu.pc = 0x4000;
    cpu.ime = false;
    cpu.halt_bug_triggered = true; // Simulating the trigger from a previous HALT

    let inc_a_opcode = OPCODES[INC_A as usize].unwrap();
    assert_eq!(inc_a_opcode.mnemonic, Mnemonic::INC);
    // 2. Place an 'INC A' (INC_A) at 0x4000
    // And place a 'DEC A' (0x3D) at 0x4001
    bus.force_write_bytes(cpu.pc, &[INC_A, 0x3D]);

    cpu.a = 5;

    // 3. First Step: Should execute INC A but PC stays at 0x4000
    let cycles = cpu.step(&mut bus);
    assert_eq!(4, cycles);
    assert_eq!(
        cpu.pc, 0x4000,
        "PC should NOT have moved forward (Halt Bug)"
    );
    assert!(
        !cpu.halt_bug_triggered,
        "Halt bug flag should clear after one use"
    );
    assert_eq!(cpu.a, 6, "Instruction INC A should have executed"); // Fails here.

    // 4. Second Step: Should execute INC A AGAIN because PC is still 0x4000
    cpu.step(&mut bus);
    assert_eq!(
        cpu.a, 7,
        "Instruction INC A should have executed a second time"
    );
    assert_eq!(cpu.pc, 0x4001, "PC should move forward normally now");
}
#[test]
fn test_ei_invincibility_window() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Setup: Interrupt is already pending, but IME is off
    cpu.pc = 0x100;
    cpu.ime = false;
    bus.write_ie(0x01); // IE: V-Blank enabled
    bus.write_if(0x01); // IF: V-Blank pending

    // 2. Execute EI
    bus.force_write_byte(0x100, 0xFB); // EI
    cpu.step(&mut bus);

    // PC should be 0x101. Interrupt should NOT have fired yet.
    assert_eq!(
        cpu.pc, 0x101,
        "Interrupt should not hijack the EI instruction itself"
    );
    assert_eq!(cpu.ime_scheduled, 1);

    // 3. Execute NOP at 0x101
    bus.force_write_byte(0x101, 0x00);
    let cycles = cpu.step(&mut bus);
    assert_eq!(4, cycles);

    // PC should be 0x102. IME is now true.
    // The interrupt still shouldn't have fired because the "Instruction after EI"
    // is protected.
    assert_eq!(
        cpu.pc, 0x102,
        "Interrupt should not hijack the instruction immediately following EI"
    );
    assert!(cpu.ime);

    // 4. Next Step: The Hardware Hijack
    cpu.step(&mut bus);

    // PC should be the START of the vector, not the byte after.
    assert_eq!(
        cpu.pc, 0x0040,
        "CPU should be sitting at the V-Blank vector"
    );

    // 5. Execute first instruction of the ISR
    bus.force_write_byte(0x0040, 0x00); // Put a NOP at the vector
    cpu.step(&mut bus);
    assert_eq!(
        cpu.pc, 0x0041,
        "CPU should have now executed the first byte of the ISR"
    );
}
#[test]
fn test_interrupt_masking() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.ime = true;
    bus.write_ie(0x01); // IE: Only V-Blank (bit 0)
    bus.write_if(0x02); // IF: LCD Stat (bit 1) requested

    // Step the CPU
    cpu.pc = 0x200;
    bus.force_write_byte(0x200, 0x00); // NOP
    cpu.step(&mut bus);

    assert_eq!(
        cpu.pc, 0x201,
        "Should NOT jump because LCD Stat is not enabled in IE"
    );
}
#[test]
fn test_halt_bug_multi_byte_shift() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.pc = 0x4000;
    cpu.ime = false;
    cpu.halt_bug_triggered = true;

    // 0x3E is 'LD A, n8'.
    // It normally reads 0x3E, then reads the next byte as data.
    bus.force_write_bytes(cpu.pc, &[0x3E, 0xFF]);

    cpu.a = 0;

    // EXECUTION:
    // 1. Fetch 0x3E. PC does NOT increment (stays at 0x4000).
    // 2. LD A, n8 needs a byte. It reads bus.read(PC).
    // 3. Since PC is 0x4000, it reads 0x3E AGAIN.
    cpu.step(&mut bus);

    assert_eq!(
        cpu.a, 0x3E,
        "A should contain the OPCODE, not the DATA, because of the PC shift"
    );
    assert_eq!(
        cpu.pc, 0x4001,
        "PC should end up at 0x4001 (only incremented once for the operand)"
    );
}
#[test]
fn test_timer_full_lifecycle() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Setup: Speed 01 (16 T-cycles per increment)
    bus.write_byte(ADDR_TIMER_TAC, 0x05);
    bus.write_byte(ADDR_TIMER_TIMA, 0xFE);
    bus.write_byte(ADDR_TIMER_TMA, 0xAA);

    let mut total_cycles = 0;

    // --- PHASE 1: Increment from 254 to 255 ---
    while total_cycles < 16 {
        let cycles = cpu.step(&mut bus);
        assert_eq!(cycles, 4);
        bus.tick_components(cycles);
        total_cycles += cycles;
    }
    assert_eq!(
        bus.read_byte(ADDR_TIMER_TIMA),
        0xFF,
        "TIMA should be 0xFF after 16+ cycles (Total: {})",
        total_cycles
    );

    // --- PHASE 2: Overflow (255 -> 0x00) ---
    let start_of_phase_2 = total_cycles;
    while total_cycles < (start_of_phase_2 + 16) {
        let cycles = cpu.step(&mut bus);
        bus.tick_components(cycles);
        total_cycles += cycles;
    }

    // Note: On real hardware, there is a 4-cycle window where TIMA is 0x00 before reload.
    // If your timer implements this delay, TIMA might be 0x00 or 0xAA depending on the exact cycle.
    let tima_val = bus.read_byte(ADDR_TIMER_TIMA);
    assert!(
        tima_val == 0x00 || tima_val == 0xAA,
        "TIMA should be 0x00 or reloaded to 0xAA (Got: 0x{:02X})",
        tima_val
    );

    // --- PHASE 3: Ensure Interrupt and Reload ---
    // Execute a few more cycles to clear any internal PPU/Timer delays
    for _ in 0..4 {
        let cycles = cpu.step(&mut bus);
        bus.tick_components(cycles);
    }

    assert_eq!(
        bus.read_tima(),
        0xAA,
        "TIMA should definitely be 0xAA (TMA) now"
    );
    assert!(
        bus.read_if() & 0x04 != 0,
        "Timer Interrupt flag (bit 2) should be set in IF"
    );
}
#[test]
fn test_timer_interrupt_trigger_robust() {
    let (mut cpu, mut bus) = bootstrap();

    // Setup: Speed 01 (16 cycles), TIMA 254
    bus.write_byte(0xFF07, 0x05);
    bus.write_byte(0xFF05, 0xFE);

    let mut total_cycles = 0;

    // Loop until we expect one increment (16 cycles)
    while total_cycles < 16 {
        let cycles = cpu.step(&mut bus);
        total_cycles += cycles;
        bus.tick_components(cycles);
    }

    assert_eq!(
        bus.read_tima(),
        0xFF,
        "After {} cycles, TIMA should be 0xFF",
        total_cycles
    );

    // Reset accumulator and loop for the next 16 cycles to trigger overflow
    total_cycles = 0;
    while total_cycles < 16 {
        let cycles = cpu.step(&mut bus);
        total_cycles += cycles;
    }

    // After overflow, TIMA should hit 0x00 (or the reload value)
    let tima_after = bus.read_byte(0xFF05);
    println!("TIMA after overflow cycles: 0x{:02X}", tima_after);
}
#[test]
fn test_timer_interrupt_trigger_detailed() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Setup
    bus.write_byte(0xFF07, 0x05); // Enable, Speed: 16 cycles
    bus.write_byte(0xFF05, 0xFE); // TIMA = 254
    bus.write_byte(0xFF06, 0xAA); // TMA = 170

    // 2. Initial state check
    assert_eq!(bus.read_byte(0xFF05), 0xFE, "TIMA should start at 0xFE");
    assert_eq!(
        bus.read_byte(0xFF0F) & 0x04,
        0,
        "IF Timer bit should be 0 initially"
    );

    // 3. Step until just before overflow (16 cycles)
    // Assuming a NOP or similar takes 4 T-cycles, 4 steps = 16 cycles
    for _ in 0..4 {
        let cycles = cpu.step(&mut bus);
        bus.tick_components(cycles);
    }
    assert_eq!(
        bus.read_byte(0xFF05),
        0xFF,
        "TIMA should be 0xFF after 16 cycles"
    );
    assert_eq!(
        bus.read_byte(0xFF0F) & 0x04,
        0,
        "IF should still be 0 at 0xFF"
    );

    // 4. Step to trigger overflow (another 16 cycles)
    for _ in 0..4 {
        let cycles = cpu.step(&mut bus);
        bus.tick_components(cycles);
    }

    // AT THIS POINT: TIMA has just hit 0x00.
    // In many implementations, this is the "delay" cycle.
    let tima_now = bus.read_tima();
    let if_now = bus.read_byte(0xFF0F);

    println!(
        "State after overflow: TIMA=0x{:02X}, IF=0x{:02X}",
        tima_now, if_now
    );

    // 5. Final Step to clear the internal delay
    cpu.step(&mut bus);

    // 6. Final Verifications
    assert!(
        bus.read_byte(0xFF0F) & 0x04 != 0,
        "Timer interrupt bit (2) MUST be set in IF. Current IF: 0x{:02X}",
        bus.read_byte(0xFF0F)
    );
    assert_eq!(
        bus.read_byte(0xFF05),
        0xAA,
        "TIMA should have reloaded from TMA (0xAA)"
    );
}
#[test]
fn test_timer_interrupt_trigger() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Enable Timer at fastest speed (4MHz / 16)
    // TAC: Bit 2 (Enable) = 1, Bits 0-1 (Speed 01) = 1 -> 0b101 (0x05)
    bus.write_byte(0xFF07, 0x05);
    bus.write_byte(0xFF05, 0xFE); // Set TIMA near overflow
    bus.write_byte(0xFF06, 0xAA); // Set TMA reload value

    // 2. Step the CPU (or just the timer) for enough cycles to overflow
    // Fastest speed is 16 cycles. If your step() increments cycles:
    for _ in 0..10 {
        let cycles = cpu.step(&mut bus);
        bus.tick_components(cycles);
    }

    // 3. Verify
    let if_reg = bus.read_byte(0xFF0F);
    assert!(
        if_reg & 0x04 != 0,
        "Timer interrupt bit (2) should be set in IF"
    ); // Failure here. left 2, right 1.
    assert_eq!(
        bus.read_byte(0xFF05),
        0xAA,
        "TIMA should have reloaded from TMA"
    );
}
#[test]
fn test_log_alignment_interrupt_hijack() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Initial State from Log 151345
    cpu.pc = 0xC2BE;
    cpu.sp = 0xDFFD;
    cpu.a = 0x04;
    cpu.b = 0x01;
    cpu.ime = true;

    // Enable Timer Interrupt in IE
    bus.write_ie(0x04);

    // 2. Setup Memory
    // C2BE: LDH (0xFF0F), A  -> This triggers the interrupt
    // C2C0: DEC B -> This should be "skipped" (pushed to stack)
    bus.force_write_bytes(cpu.pc, &[0xE0, 0x0F, 0x05]); // LDH (0xFF0F), A and DEC B

    // 0050: INC A -> First instruction of ISR
    bus.force_write_byte(0x0050, INC_A);

    // --- STEP 1: Execute LDH ---
    cpu.step(&mut bus);
    // After this, PC should be C2C0, and IF bit 2 should be set.
    assert_eq!(cpu.pc, 0xC2C0);
    assert_eq!(
        bus.read_if() & 0x04,
        0x04,
        "Timer interrupt should be pending"
    );

    // --- STEP 2: The Hijack Step ---
    cpu.step(&mut bus);

    // Assertions based on "Expected" log 151347
    assert_eq!(
        cpu.pc, 0x0051,
        "PC should be at 0x051 (Vector 0x050 + INC A executed)"
    );
    assert_eq!(cpu.a, 0x05, "A should be 0x05 (INC A executed)");
    assert_eq!(
        cpu.sp, 0xDFFB,
        "SP should be DFFB (PC C2C0 pushed to stack)"
    );

    let stack_low = bus.read_byte(0xDFFB);
    let stack_high = bus.read_byte(0xDFFC);
    assert_eq!(stack_low, 0xC0);
    assert_eq!(stack_high, 0xC2);
}
#[test]
fn test_handle_interrupts_return_state() {
    let (mut cpu, mut bus) = bootstrap();

    // Setup state before interrupt
    cpu.pc = 0xC2C0;
    cpu.ime = true;
    bus.write_ie(0x04); // IE: Timer
    bus.write_if(0x04); // IF: Timer

    // Call the function
    let result = cpu.handle_interrupts(&mut bus);

    // 1. Check Flow Control
    match result {
        StepFlowController::EarlyReturn(cycles) => assert_eq!(cycles, 20),
        _ => panic!("Expected EarlyReturn(20)"),
    }

    // 2. Check side effects
    assert_eq!(cpu.pc, 0x0050, "PC should be at Timer Vector");
    assert_eq!(cpu.ime, false, "IME should be disabled after service");
    assert_eq!(bus.read_byte(0xFF0F) & 0x04, 0, "IF bit should be cleared");
}
#[test]
fn test_halt_bug_pc_behavior() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.pc = 0xC000;
    cpu.ime = false; // IME must be OFF for the bug
    let initial_a = cpu.a;

    // 1. Setup HALT followed by a NOP
    bus.force_write_bytes(cpu.pc, &[HALT, INC_A]); // INC_A will be affected.

    // 2. Make an interrupt pending
    bus.write_ie(0x01); // V-Blank
    bus.write_if(0x01); // V-Blank

    // 3. Step once (Executes HALT)
    cpu.step(&mut bus);

    // In the HALT BUG, the CPU doesn't stop,
    // and it fails to increment PC for the NEXT instruction.
    assert!(!cpu.halted, "CPU should not be halted due to HALT bug");
    assert_eq!(cpu.pc, 0xC001, "PC should point to INC A");

    // 4. Step again (Executes INC A)
    cpu.step(&mut bus);

    // THE BUG: The PC should still be 0xC001 because the increment was skipped!
    assert_eq!(
        cpu.pc, 0xC001,
        "HALT Bug failed: PC should not have advanced after INC A"
    );
    assert_eq!(cpu.a, initial_a + 1, "INC A should have executed once");

    // 5. Step again (Executes INC A a second time)
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0xC002, "PC should finally advance now");
    assert_eq!(
        cpu.a,
        initial_a + 2,
        "INC A should have executed twice total"
    );
}
#[test]
fn test_halt_bug_lifecycle() {
    let (mut cpu, mut bus) = bootstrap();

    // Explicitly initialize state for a clean test
    cpu.a = 0;
    cpu.pc = 0xC000;
    cpu.ime = false;

    bus.force_write_bytes(cpu.pc, &[HALT, INC_A]);

    // Trigger condition for HALT Bug: IME=0 and (IE & IF) != 0
    bus.write_ie(0x01); // IE: V-Blank enabled
    bus.write_if(0x01); // IF: V-Blank pending

    // --- Step 1: Execute HALT ---
    let cycles = cpu.step(&mut bus);
    bus.tick_components(cycles);

    // The CPU should NOT enter the halted state, but the bug should be primed
    assert!(cpu.halt_bug_triggered, "Halt bug should be primed");
    assert!(
        !cpu.halted,
        "CPU should not actually halt when an interrupt is pending and IME=0"
    );
    assert_eq!(cpu.pc, 0xC001, "PC should move to the byte after HALT");

    // --- Step 2: Execute INC A (First Time) ---
    // Because of the bug, the CPU fetches INC A but fails to increment the PC.
    let cycles = cpu.step(&mut bus);
    bus.tick_components(cycles);

    assert_eq!(cpu.a, 1, "INC A should have executed once");
    assert_eq!(
        cpu.pc, 0xC001,
        "PC MUST NOT increment! This is the essence of the Halt Bug."
    );
    assert!(
        !cpu.halt_bug_triggered,
        "The bug flag should clear after the failed increment"
    );

    // --- Step 3: Execute INC A (Second Time) ---
    // The PC is still at 0xC001, so the CPU fetches and executes INC A again.
    let cycles = cpu.step(&mut bus);
    bus.tick_components(cycles);

    assert_eq!(cpu.a, 2, "INC A should have executed a second time");
    assert_eq!(
        cpu.pc, 0xC002,
        "PC should now finally move forward normally"
    );
}

#[test]
fn test_halt_no_bug_if_ime_on() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC000;
    cpu.ime = true; // IME is ON

    bus.force_write_bytes(cpu.pc, &[HALT, INC_A]);
    bus.write_ie(0x01);
    bus.write_if(0x01); // IF (Interrupt is pending!)

    cpu.step(&mut bus);

    // Because IME is ON, it should NOT trigger the halt bug.
    // It should service the interrupt instead (PC jumps to vector).
    assert!(!cpu.halt_bug_triggered);
    assert_ne!(cpu.pc, 0xC001, "Should have jumped to interrupt vector");
}
#[test]
fn test_halt_wakeup_and_stay_awake() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.pc = 0xC000;
    cpu.ime = false;

    // Default is NOP, but i guess it's better to be explicit.
    bus.force_write_bytes(cpu.pc, &[HALT, NOP, NOP]);

    // 1. Execute HALT
    cpu.step(&mut bus);
    assert!(cpu.halted, "Should be halted now");

    // 2. Trigger interrupt to wake it up
    bus.write_ie(0x01);
    bus.write_if(0x01);

    // 3. This step should wake up and execute the NOP at C001
    cpu.step(&mut bus);
    assert!(!cpu.halted, "Should have woken up");
    assert_eq!(cpu.pc, 0xC002, "Should have moved past the first NOP");

    // 4. This step should execute the NOP at C002
    // If your bug exists, this will return "early exit" instead!
    cpu.step(&mut bus);
    assert_eq!(cpu.pc, 0xC003, "Should have moved past the second NOP");
}
#[test]
fn test_halt_prohibit_immediate_rehalt() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.pc = 0xC000;
    bus.force_write_bytes(cpu.pc, &[HALT, NOP]);

    // 1. Execute HALT
    cpu.step(&mut bus);
    assert!(cpu.halted);
    // PC should have incremented to C001 after fetching the HALT
    assert_eq!(cpu.pc, 0xC001);

    // 2. Wake up
    bus.write_ie(0x01);
    bus.write_if(0x01);

    cpu.step(&mut bus); // Should execute NOP
    assert!(!cpu.halted, "CPU should be awake");
    assert_eq!(cpu.pc, 0xC002, "PC should have moved to C002");

    // 3. Next step should NOT be a halt exit
    let result = cpu.handle_halt_logic(&mut bus);
    assert!(matches!(result, StepFlowController::Continue));
}
#[test]
fn test_halt_bug_step_isolation() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC000;
    cpu.ime = false;
    let initial_a = cpu.a;

    bus.force_write_bytes(cpu.pc, &[HALT, INC_A]);
    bus.write_ie(0x01); // IE: V-Blank enabled
    bus.write_if(0x01); // IF: V-Blank pending

    // Execute exactly ONE step. This should ONLY execute HALT.
    cpu.step(&mut bus);

    assert_eq!(
        cpu.a, initial_a,
        "A should STILL be initial value, if it's not, HALT is executing the next op immediately."
    );
    assert_eq!(
        cpu.pc, 0xC001,
        "PC should have moved to the next byte (INC A)"
    );
    assert!(
        cpu.halt_bug_triggered,
        "Bug should be armed for the NEXT step"
    );
}
#[test]
fn test_halt_bug_step_by_step() {
    let (mut cpu, mut bus) = bootstrap();

    cpu.pc = 0xC000;
    cpu.ime = false;
    bus.force_write_bytes(cpu.pc, &[INC_A]);
    bus.write_ie(0x01); // IE: V-Blank enabled
    bus.write_if(0x01); // IF: V-Blank pending
    assert_eq!(bus.pending_interrupt(), true);

    cpu.halt(OPCODES[HALT as usize].unwrap(), &mut bus);

    assert!(cpu.halt_bug_triggered, "Flag must be true now");

    cpu.fetch_and_execute(&mut bus);

    assert_eq!(cpu.pc, 0xC000, "HALT BUG: PC should NOT have incremented!");
    assert!(
        !cpu.halt_bug_triggered,
        "Flag should have been cleared by fetch_byte"
    );

    let opcode3 = cpu.fetch_byte(&mut bus);
    assert_eq!(opcode3, INC_A, "Should fetch INC A again");
    assert_eq!(cpu.pc, 0xC001, "Now PC should finally increment to C002");

    assert_eq!(cpu.a, 2, "A should be 2 after the double execution");
}
#[test]
fn test_halt_pc_movement_only() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC36F;
    bus.force_write_bytes(cpu.pc, &[HALT, NOP]);

    // We use the same conditions as your log (IME=0, IF=0)
    cpu.ime = false;
    bus.force_write_byte(0xFF0F, 0x00);

    cpu.step(&mut bus);

    // After HALT, PC should be exactly one byte forward.
    assert_eq!(
        cpu.pc, 0xC370,
        "PC should move from C36F to C370 after HALT fetch"
    );
    assert!(cpu.halted, "CPU should be halted");
}
#[test]
fn test_halt_bug_pc_locking() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC000;
    cpu.ime = false;

    bus.force_write_byte(cpu.pc, HALT);
    bus.write_ie(0x01);
    bus.write_if(0x01); // Bug triggered!

    // 1. Fetch the HALT
    let _op = cpu.fetch_byte(&mut bus);
    assert_eq!(cpu.pc, 0xC001, "PC must move to C001 after fetching HALT");

    // 2. Execute HALT
    let info = OPCODES[HALT as usize].unwrap();
    assert_eq!(info.mnemonic, Mnemonic::HALT);
    cpu.halt(info, &mut bus);
    assert!(cpu.halt_bug_triggered);

    // 3. The NEXT fetch (the bugged one)
    let _next_op = cpu.fetch_byte(&mut bus);
    assert_eq!(cpu.pc, 0xC001, "BUG: PC should NOT move during this fetch!");

    // 4. The THIRD fetch (the recovery)
    let _final_op = cpu.fetch_byte(&mut bus);
    assert_eq!(cpu.pc, 0xC002, "PC should finally move to C002 now");
}
#[test]
fn test_manual_bug_execution() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC000;
    cpu.ime = false;
    let initial_a = cpu.a;

    bus.force_write_bytes(cpu.pc, &[HALT, INC_A]);
    bus.write_ie(0x01);
    bus.write_if(0x01);

    // 1. Manually run the first instruction (HALT)
    cpu.fetch_and_execute(&mut bus);
    assert!(cpu.halt_bug_triggered);
    assert_eq!(cpu.pc, 0xC001);

    // 2. Manually run the second instruction (The first INC A)
    cpu.fetch_and_execute(&mut bus);
    assert_eq!(
        cpu.a,
        initial_a + 1,
        "A should be 1 after one fetch_and_execute"
    );
    assert_eq!(cpu.pc, 0xC001, "PC should STILL be C001");

    // 3. Manually run the third instruction (The second INC A)
    cpu.fetch_and_execute(&mut bus);
    assert_eq!(
        cpu.a,
        initial_a + 2,
        "A should be 2 after second fetch_and_execute"
    );
    assert_eq!(cpu.pc, 0xC002, "PC should finally be C002");
}
#[test]
fn test_fetch_byte_bug_isolation() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC000;

    // Arm the bug manually
    cpu.halt_bug_triggered = true;
    bus.force_write_byte(cpu.pc, INC_A);

    // First fetch: should NOT increment PC
    let op1 = cpu.fetch_byte(&mut bus);
    assert_eq!(op1, INC_A);
    assert_eq!(cpu.pc, 0xC000, "PC should not have moved!");
    assert!(!cpu.halt_bug_triggered, "Flag should be reset");

    // Second fetch: should increment PC
    let op2 = cpu.fetch_byte(&mut bus);
    assert_eq!(op2, INC_A);
    assert_eq!(cpu.pc, 0xC001, "PC should move now");
}
#[test]
fn test_pc_and_flag_alignment() {
    let (mut cpu, mut bus) = bootstrap();
    cpu.pc = 0xC000;
    cpu.ime = false;

    bus.force_write_bytes(cpu.pc, &[HALT, INC_A]);
    bus.write_ie(0x01);
    bus.write_if(0x01);

    // Step 1: Execute HALT
    cpu.fetch_and_execute(&mut bus);
    // PC should be C001, Bug should be true
    assert_eq!(cpu.pc, 0xC001);
    assert!(cpu.halt_bug_triggered);

    // Step 2: Execute INC A
    cpu.fetch_and_execute(&mut bus);
    // PC should be C001 (because fetch_byte skipped increment)
    // BUT! Did your dispatch/length-adder move it to C002?

    println!("PC after first INC A: {:04X}", cpu.pc);
    println!("Bug Flag after first INC A: {}", cpu.halt_bug_triggered);
}
#[test]
fn test_ldh_a_n8_regression() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Prepare the High RAM (HRAM) value
    // In your log, LDH A, (a8) was expected to result in A=0x90.
    // The instruction hex was F0 44 (LDH A, ($44))
    let hram_addr = 0xFF44;
    let expected_val = 0x90;

    // Write the value to the bus first
    bus.write_byte(hram_addr, expected_val);

    // 1. Prepare the High RAM (HRAM) value
    // let hram_addr = 0xFF44;
    let expected_val = 0x90;

    // INSTEAD OF: bus.write_byte(hram_addr, expected_val);
    // DO THIS:
    bus.ppu.set_ly(expected_val);

    // 2. Set up CPU state to match log 16508
    cpu.pc = 0xC7F3;
    cpu.a = 0xFF; // A was 0xFF before the instruction

    // Manually place the instruction in memory
    // F0 = Opcode for LDH A, (n8)
    // 44 = The immediate operand (offset)
    bus.force_write_bytes(cpu.pc, &[0xF0, 0x44]);

    // 3. Execute the instruction
    // This should fetch 0xF0, then 0x44, then read from 0xFF44
    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12, "LDH A, (n8) incorrect T-cycle count");

    // 4. Verification
    assert_eq!(
        cpu.a, expected_val,
        "LDH A, (n8) failed: Expected A to be 0x90, but got 0x{:02X}",
        cpu.a
    );
    assert_eq!(cpu.pc, 0xC7F5, "PC should have advanced by 2 bytes");
}

#[test]
fn test_vblank_interrupt_trigger() {
    let (mut cpu, mut bus) = bootstrap();

    // 1. Setup: Enable V-Blank interrupts
    bus.write_ie(0x01);
    cpu.ime = true; // Master Interrupt Enable

    // 2. Simulate PPU reaching V-Blank
    // Instead of hardcoding read_byte, we simulate the PPU ticking
    // from the end of LY 143 to the start of LY 144.
    bus.ppu.ly = 143;
    bus.ppu.dot_counter = 452;

    // Tick the PPU enough to push it into LY 144
    bus.tick_components(8);

    assert_eq!(bus.ppu.ly, 144, "PPU should have reached LY 144");

    assert_eq!(
        bus.read_if(),
        0x01,
        "IF V-Blank bit should be set when LY hits 144"
    );

    // 4. Step CPU and check if it jumped to 0x0040 (V-Blank Vector)
    // A standard interrupt takes 5 M-cycles (20 T-cycles)
    cpu.step(&mut bus);

    assert_eq!(
        cpu.pc, ADDR_VEC_VBLANK,
        "CPU should have jumped to V-Blank interrupt vector"
    );
}
