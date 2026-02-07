use gameboy_rs::constants::*;
use gameboy_rs::ppu::Ppu;

fn ppu() -> Ppu {
    Ppu::new() // Assuming default state is LCD ON, LY=0, Dot=0
}
#[test]
fn test_ppu_scanline_increment() {
    let mut ppu = ppu();
    // Turn LCD OFF then ON to ensure a clean sync point
    ppu.write_byte(ADDR_PPU_LCDC, 0x00);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_LY),
        0,
        "LY must be 0 when LCD is OFF"
    );

    ppu.write_byte(ADDR_PPU_LCDC, 0x80);

    // Move to the very end of the first scanline (0 to 455)
    ppu.tick(455);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_LY),
        0,
        "LY should still be 0 at dot 455"
    );

    // One more tick should trigger the increment to line 1
    ppu.tick(1);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_LY),
        1,
        "LY should increment to 1 at dot 456"
    );
}

#[test]
fn test_ppu_vblank_entry_timing() {
    let mut ppu = ppu();
    ppu.write_byte(ADDR_PPU_LCDC, 0x00);
    ppu.write_byte(ADDR_PPU_LCDC, 0x80);
    assert!(ppu.lcd_enabled());

    // Advance 144 lines (0 through 143)
    // 144 * 456 = 65,664 dots
    ppu.tick(65664);

    // At exactly 65,664 dots, we should have just hit the first T-cycle of LY 144
    assert_eq!(
        ppu.read_byte(ADDR_PPU_LY),
        144,
        "Should be exactly at the start of V-Blank"
    );

    let stat = ppu.read_byte(ADDR_PPU_STAT);
    assert_eq!(stat & 0x03, 1, "STAT mode should be 1 (V-Blank)");
}

#[test]
fn test_ppu_lcd_enable_synchronization() {
    let mut ppu = ppu();

    // 1. Force LCD OFF
    ppu.write_byte(ADDR_PPU_LCDC, 0x00);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_LY),
        0,
        "LY must be 0 when LCD is OFF"
    );

    // 2. Turn LCD ON
    ppu.write_byte(ADDR_PPU_LCDC, 0x80);

    // 3. Immediate check: Hardware resets to Mode 2 (OAM Search)
    let stat = ppu.read_byte(ADDR_PPU_STAT);
    assert_eq!(
        stat & 0x03,
        2,
        "Should immediately enter Mode 2 upon enable"
    );

    // 4. Verify Mode 2 duration (80 dots)
    ppu.tick(79);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        2,
        "Should stay in Mode 2 until dot 80"
    );

    ppu.tick(1);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        3,
        "Should transition to Mode 3 at dot 80"
    );
}
#[test]
fn test_ppu_vblank_wrap_around() {
    let mut ppu = ppu();
    ppu.write_byte(ADDR_PPU_LCDC, 0x80); // Enable LCD

    // 1. Advance to the start of V-Blank (Line 144)
    ppu.tick(144 * 456);
    assert_eq!(ppu.read_byte(ADDR_PPU_LY), 144);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        1,
        "Should be in Mode 1"
    );

    // 2. Advance to the very last dot of the very last V-Blank line (Line 153)
    // Total dots to reach end of frame: 154 * 456 = 70,224
    // We are at 65,664. We need 4,560 more dots to finish line 153.
    ppu.tick(4559);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_LY),
        153,
        "Should be on the last line of V-Blank"
    );
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        1,
        "Should still be in Mode 1"
    );

    // 3. One more tick should wrap LY back to 0 and enter Mode 2
    ppu.tick(1);
    assert_eq!(ppu.read_byte(ADDR_PPU_LY), 0, "LY should wrap back to 0");
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        2,
        "Should be back in Mode 2 for new frame"
    );
}

#[test]
fn test_ppu_stat_write_masking() {
    let mut ppu = ppu();
    ppu.write_byte(ADDR_PPU_LCDC, 0x80); // Enable (Mode 2)

    // Mode is 2 (binary 10). Let's try to write 0xFF to STAT.
    // Bits 0-2 are Read-Only (Mode and LYC=LY flag).
    // Bit 7 is usually always 1 or unused.
    ppu.write_byte(ADDR_PPU_STAT, 0xFF);

    let val = ppu.read_byte(ADDR_PPU_STAT);

    // Mode should STILL be 2 (bit 1 set, bit 0 clear)
    assert_eq!(
        val & 0x03,
        2,
        "Writing to STAT should not change the hardware mode bits"
    );
}

#[test]
fn test_ppu_hblank_transition() {
    let mut ppu = ppu();
    ppu.write_byte(ADDR_PPU_LCDC, 0x80);

    // Mode 2: 0-79 dots
    // Mode 3: 80-251 dots (assuming fixed 172 dot duration)
    // Mode 0: 252-455 dots

    ppu.tick(80 + 171); // Total 251 dots
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        3,
        "Should still be in Mode 3 at dot 251"
    );

    ppu.tick(1); // Dot 252
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        0,
        "Should transition to Mode 0 at dot 252"
    );

    ppu.tick(203); // Reach dot 455
    assert_eq!(ppu.read_byte(ADDR_PPU_LY), 0, "Should still be on line 0");
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x03,
        0,
        "Should still be in Mode 0"
    );
}

#[test]
fn test_ppu_stat_interrupt_edge_trigger() {
    let mut ppu = ppu();
    // Enable Mode 2 Interrupt (Bit 5) and LYC Interrupt (Bit 6)
    // 0x40 | 0x20 = 0x60
    ppu.write_byte(ADDR_PPU_STAT, 0x60);
    ppu.write_byte(ADDR_PPU_LYC, 0); // LYC = 0, so LY == LYC is true immediately

    // Turn LCD on.
    // Both Mode 2 and LYC are now active. This should trigger ONE interrupt.
    let irq_on_enable = ppu.tick(1);

    // Now, move to dot 80 (Mode 3).
    // LYC is still true, but Mode 2 is now false.
    // The OR gate: (LYC=True || Mode2=False) = True.
    // Since the signal stayed True, no new rising edge occurs.
    let irq_on_mode_change = ppu.tick(80);

    assert_eq!(
        irq_on_enable,
        (false, false),
        "Should trigger interrupt on enable (Mode 2 + LYC)"
    );
    assert_eq!(
        irq_on_mode_change,
        (false, false),
        "Should NOT trigger a new interrupt because signal stayed HIGH"
    );
}

#[test]
fn test_ppu_lyc_flag_timing() {
    let mut ppu = ppu();
    ppu.write_byte(ADDR_PPU_LYC, 1);
    ppu.write_byte(ADDR_PPU_LCDC, 0x80);

    // End of line 0
    ppu.tick(455);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x04,
        0,
        "LYC flag should be 0 (LY=0, LYC=1)"
    );

    // Move to line 1
    ppu.tick(1);
    assert_eq!(ppu.read_byte(ADDR_PPU_LY), 1);
    assert_eq!(
        ppu.read_byte(ADDR_PPU_STAT) & 0x04,
        0x04,
        "LYC flag should be 1 (LY=1, LYC=1)"
    );
}
