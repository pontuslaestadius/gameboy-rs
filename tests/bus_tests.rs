use gameboy_rs::input::DummyInput;
use gameboy_rs::mmu::{Bus, Memory};

fn bus() -> Bus<DummyInput> {
    let bus: Bus<DummyInput> = Bus::new(Vec::new()); // Your memory/system component
    bus
}

#[test]
fn test_bus_timer_interrupt_integration() {
    let mut bus = bus();

    // 1. Configure Timer via Bus writes
    bus.write_byte(0xFF06, 0xAA); // TMA = 170
    bus.write_byte(0xFF07, 0x05); // TAC = Enabled, 16-cycle mode
    bus.write_byte(0xFF05, 0xFE); // TIMA = 254

    // Clear Interrupt Flags
    bus.write_byte(0xFF0F, 0x00);

    // 2. We need 32 T-cycles to trigger two increments (254 -> 255 -> 0/Reload)
    // If your bus.tick() takes M-cycles, divide by 4.
    // Assuming bus.tick(cycles) takes T-cycles here:
    for _ in 0..32 {
        bus.tick_components(1);
    }

    // 3. Verify the chain reaction
    let tima = bus.read_byte(0xFF05);
    let if_reg = bus.read_byte(0xFF0F);

    assert_eq!(tima, 0xAA, "TIMA should have reloaded from TMA (0xAA)");
    assert!(
        if_reg & 0x04 != 0,
        "Timer interrupt bit (2) should be set in IF register"
    );
}
#[test]
fn test_timer_via_tick_components() {
    let mut bus = bus();

    // Setup: Fast timer (16 cycle mode), enabled
    bus.write_byte(0xFF07, 0x05);
    bus.write_byte(0xFF06, 0xAA); // TMA = 0xAA
    bus.write_byte(0xFF05, 0xFF); // TIMA = 0xFF (One step from overflow)
    bus.write_byte(0xFF0F, 0x00); // Clear Interrupt Flags

    // Execute 16 cycles (enough for one TIMA increment at speed 01)
    bus.tick_components(16);

    // Verify
    let tima = bus.read_byte(0xFF05);
    let if_reg = bus.read_byte(0xFF0F);

    assert_eq!(
        tima, 0xAA,
        "TIMA should have wrapped around to TMA value 0xAA"
    );
    assert_eq!(
        if_reg & 0x04,
        0x04,
        "Timer interrupt bit (2) should be set in IF register"
    );
}
#[test]
fn test_bus_div_reset_glitch() {
    let mut bus = bus();

    bus.write_byte(0xFF07, 0x05); // Enable, 16-cycle mode (Bit 3)
    bus.write_byte(0xFF05, 0x00); // TIMA = 0

    // 1. Tick 8 times. Internal counter is 8 (0b1000). Bit 3 is HIGH.
    bus.tick_components(8);
    assert_eq!(
        bus.read_byte(0xFF05),
        0,
        "TIMA should not have incremented yet"
    );

    // 2. Write to DIV to reset it.
    // This should trigger the falling edge glitch and increment TIMA.
    bus.write_byte(0xFF04, 0x00);

    assert_eq!(
        bus.read_byte(0xFF05),
        1,
        "TIMA should have incremented due to DIV reset glitch"
    );
}
#[test]
fn test_vram_bus_communication() {
    let mut bus = bus();
    let vram_addr = 0x8000;
    let test_byte = 0x55;

    // Write to VRAM via Bus
    bus.write_byte(vram_addr, test_byte);

    // Read from VRAM via Bus
    let read_val = bus.read_byte(vram_addr);

    assert_eq!(
        read_val, test_byte,
        "VRAM Read/Write mismatch! Wrote {:02X}, Read {:02X}. Check Bus routing for 0x8000..=0x9FFF",
        test_byte, read_val
    );
}
#[test]
fn test_timer_standalone_lifecycle() {
    let mut bus = bus();

    // 1. Setup Timer: Enable, Speed 01 (16 T-cycles)
    bus.write_byte(0xFF07, 0x05);
    bus.write_byte(0xFF06, 0xAA); // TMA Reload value
    bus.write_byte(0xFF05, 0xFE); // TIMA start

    // Verify initial state
    assert_eq!(bus.read_byte(0xFF05), 0xFE);

    // 2. Tick exactly 15 cycles. TIMA should NOT change yet.
    bus.timer.tick(15);
    assert_eq!(
        bus.read_byte(0xFF05),
        0xFE,
        "TIMA should not increment until 16 cycles"
    );

    // 3. Tick the 16th cycle.
    bus.timer.tick(1);
    assert_eq!(
        bus.read_byte(0xFF05),
        0xFF,
        "TIMA should be 0xFF after exactly 16 cycles"
    );

    // 4. Tick 15 more cycles. Still 0xFF.
    bus.timer.tick(15);
    assert_eq!(
        bus.read_byte(0xFF05),
        0xFF,
        "TIMA should stay 0xFF until the next 16-cycle boundary"
    );

    // 5. Tick 1 cycle to trigger overflow (255 -> 0).
    bus.tick_components(1);

    // Check state immediately after overflow
    let tima_val = bus.read_byte(0xFF05);
    // let if_reg = bus.read_if();

    // Depending on your implementation of the "4-cycle delay":
    // If you don't have a delay, this should be 0xAA and bit 2 of IF should be set.
    assert!(
        tima_val == 0x00 || tima_val == 0xAA,
        "TIMA overflowed but got 0x{:02X}",
        tima_val
    );

    // 6. Ensure reload and interrupt are processed
    bus.tick_components(4); // Extra ticks to clear any internal delay logic

    assert_eq!(
        bus.read_byte(0xFF05),
        0xAA,
        "TIMA should be reloaded with TMA (0xAA)"
    );
    assert!(
        bus.read_if() & 0x04 != 0,
        "Timer interrupt bit (2) should be set in IF"
    );
}
#[test]
fn diagnostic_timer_signal() {
    let mut bus = bus();

    bus.write_byte(0xFF07, 0x05);
    bus.write_byte(0xFF05, 0xFF); // One tick away from overflow

    // 1. Trigger the overflow
    bus.tick_components(16);

    // 2. Check internal vs external state
    let external_if_reg = bus.read_byte(0xFF0F);

    println!("Bus IF Register: 0x{:02X}", external_if_reg);

    assert!(external_if_reg & 0x04 != 0, "IF register bit 2 is NOT set");
}
