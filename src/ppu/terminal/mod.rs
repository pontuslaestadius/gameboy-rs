use crate::ppu::Ppu;

const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;

fn clear_terminal() {
    print!("{}[2J", 27 as char);
}

pub fn display_buffer(ppu: &Ppu) {
    let buffer = ppu.get_frame_buffer();

    clear_terminal();

    // We step by 2 on Y because one character represents two vertical pixels
    for y in 0..DISPLAY_HEIGHT {
        for x in 0..DISPLAY_WIDTH {
            let value = buffer[y * 160 + x];
            if value == 0 {
                print!("   ");
            } else {
                print!("{:0X} ", value);
            }
        }
        println!("");
    }
}

pub fn display_frame(ppu: &Ppu) {
    let buffer = ppu.get_frame_buffer();
    let mut output = String::with_capacity(DISPLAY_WIDTH * DISPLAY_HEIGHT);

    // Move cursor to top-left (don't clear screen to avoid flickering)
    output.push_str("\x1B[H");

    // We step by 2 on Y because one character represents two vertical pixels
    for y in (0..DISPLAY_HEIGHT).step_by(2) {
        for x in 0..DISPLAY_WIDTH {
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
    output.push_str("\x1B[H");
    // Debugging footer.
    // output.push_str("Footer");
    // output.push('\n');
    print!("{}", output);
}
