use crate::ppu::Ppu;

pub fn display_frame(ppu: &dyn Ppu) {
    let buffer = ppu.get_frame_buffer();
    let mut output = String::with_capacity(160 * 144);

    // Move cursor to top-left (don't clear screen to avoid flickering)
    output.push_str("\x1B[H");

    // We step by 2 on Y because one character represents two vertical pixels
    for y in (0..144).step_by(2) {
        for x in 0..160 {
            let top_pixel = buffer[y * 160 + x];
            let bottom_pixel = buffer[(y + 1) * 160 + x];

            // Use ANSI colors to map the 4 shades (0=White, 3=Black)
            // This is a simplified mapping for grayscale terminals
            let top_color = 232 + (top_pixel * 7);
            let bottom_color = 232 + (bottom_pixel * 7);

            // ▄ is the Unicode "Lower Half Block"
            output.push_str(&format!(
                "\x1B[38;5;{}m\x1B[48;5;{}m▄",
                bottom_color, top_color
            ));
        }
        output.push('\n');
    }
    print!("{}", output);
}
