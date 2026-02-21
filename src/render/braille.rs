use super::canvas::{Canvas, ColorMode, color_to_fg};

/// Braille dot positions within a 2x4 cell:
/// (0,0) (1,0)    dot1 dot4
/// (0,1) (1,1)    dot2 dot5
/// (0,2) (1,2)    dot3 dot6
/// (0,3) (1,3)    dot7 dot8
///
/// Unicode braille: U+2800 + dot_bits
const BRAILLE_OFFSET: u32 = 0x2800;
const DOT_MAP: [(usize, usize, u32); 8] = [
    (0, 0, 0x01), // dot 1
    (0, 1, 0x02), // dot 2
    (0, 2, 0x04), // dot 3
    (1, 0, 0x08), // dot 4
    (1, 1, 0x10), // dot 5
    (1, 2, 0x20), // dot 6
    (0, 3, 0x40), // dot 7
    (1, 3, 0x80), // dot 8
];

const THRESHOLD: f64 = 0.3;

pub fn render(canvas: &Canvas) -> String {
    let term_cols = canvas.width / 2;
    let term_rows = canvas.height / 4;
    let mut out = String::with_capacity(term_cols * term_rows * 20);

    for row in 0..term_rows {
        for col in 0..term_cols {
            let px = col * 2;
            let py = row * 4;

            let mut bits: u32 = 0;
            let mut total_r: u32 = 0;
            let mut total_g: u32 = 0;
            let mut total_b: u32 = 0;
            let mut lit_count: u32 = 0;

            for &(dx, dy, bit) in &DOT_MAP {
                let x = px + dx;
                let y = py + dy;
                if x < canvas.width && y < canvas.height {
                    let idx = y * canvas.width + x;
                    if canvas.pixels[idx] > THRESHOLD {
                        bits |= bit;
                        let (r, g, b) = canvas.colors[idx];
                        total_r += r as u32;
                        total_g += g as u32;
                        total_b += b as u32;
                        lit_count += 1;
                    }
                }
            }

            let ch = char::from_u32(BRAILLE_OFFSET + bits).unwrap_or(' ');

            if canvas.color_mode != ColorMode::Mono && lit_count > 0 {
                let r = (total_r / lit_count) as u8;
                let g = (total_g / lit_count) as u8;
                let b = (total_b / lit_count) as u8;
                let color = canvas.map_color(r, g, b);
                out.push_str(&format!("\x1b[{}m{}", color_to_fg(color), ch));
            } else {
                out.push(ch);
            }
        }
        if canvas.color_mode != ColorMode::Mono {
            out.push_str("\x1b[0m");
        }
        // Use cursor movement instead of \n to avoid blank line issues
        out.push_str(&format!("\x1b[{};1H", row + 2)); // move to next row (1-indexed)
    }
    out
}
