use gameboy_rs::apu::Apu;

// #[test]
// fn test_apu_power_state() {
//     let mut apu = Apu::new();
//     assert!(!apu.enabled());
//     assert_eq!(apu.nr52, 0);
//     apu.set_power_state(true);
//     assert_eq!(apu.nr52, 0x80);
//     assert!(apu.enabled());
//     apu.set_power_state(false);
//     assert!(!apu.enabled());
//     assert_eq!(apu.nr52, 0);
// }

// #[test]
// fn test_apu_len_ctr_status_clearing() {
//     let mut apu = Apu::new();

//     // 1. Setup: Enable Channel 1 and set a small length
//     // NR11 (0xFF11): Bits 0-5 are length (0-63).
//     // We'll set it so it expires quickly.
//     apu.write_byte(0xFF11, 60);

//     // 2. Start the channel with Length Enable (NR14 Bit 6)
//     apu.write_byte(0xFF14, 0x80 | 0x40);

//     // 3. Verify status bit is set in NR52 (0xFF26)
//     assert!(
//         (apu.read_byte(0xFF26) & 0x01) != 0,
//         "Channel 1 status should be active"
//     );

//     // 4. Tick the APU/Frame Sequencer enough times to expire the length
//     // (This depends on your specific tick implementation)
//     for _ in 0..10000 {
//         apu.tick(1);
//     }

//     // PROOF OF ERROR: If the test fails here, your NR52 status is not
//     // correctly tracking the internal state of the length counter.
//     assert_eq!(
//         apu.read_byte(0xFF26) & 0x01,
//         0,
//         "Channel 1 status bit did not clear after length expired"
//     );
// }

// #[test]
// fn test_apu_auto_expiration_via_tick() {
//     let mut apu = Apu::new();

//     // 1. Setup Channel 1: Minimum length, Length Enable ON
//     apu.write_byte(0xFF11, 0x3F); // Length = 1 (very short)
//     apu.write_byte(0xFF14, 0xC0); // Initial + Length Enable

//     // 2. Verify it's initially ON
//     assert!((apu.read_byte(0xFF26) & 0x01) != 0);

//     // 3. Tick the APU for a large number of cycles
//     // (A length of 1 at 256Hz expires in roughly 16,384 T-cycles)
//     for _ in 0..20000 {
//         apu.tick(1);
//     }

//     // PROOF OF ERROR: If no tick exists, the channel stays ON forever.
//     let status = apu.read_byte(0xFF26);
//     assert_eq!(
//         status & 0x01,
//         0,
//         "Channel 1 failed to turn off automatically via tick"
//     );
// }
