use super::canvas::{Canvas, ColorMode};
use super::cell::{Cell, CellGrid};

/// Braille dot positions within a 2x4 cell:
/// (0,0) (1,0)    dot1 dot4
/// (0,1) (1,1)    dot2 dot5
/// (0,2) (1,2)    dot3 dot6
/// (0,3) (1,3)    dot7 dot8
const BRAILLE_OFFSET: u32 = 0x2800;
const DOT_MAP: [(usize, usize, u32); 8] = [
    (0, 0, 0x01),
    (0, 1, 0x02),
    (0, 2, 0x04),
    (1, 0, 0x08),
    (1, 1, 0x10),
    (1, 2, 0x20),
    (0, 3, 0x40),
    (1, 3, 0x80),
];

/// Minimum pixel brightness [0.0..=1.0] to render a braille dot.
/// Pixels at or below this value are treated as dark/empty.
/// Calibrated so mid-intensity animations fill ~50% of dots.
const BRIGHTNESS_THRESHOLD: f64 = 0.3;

pub fn render(canvas: &Canvas) -> String {
    super::encoder::encode_full(&build_grid(canvas), true)
}

pub fn build_grid(canvas: &Canvas) -> CellGrid {
    let cols = canvas.width / 2;
    let rows = canvas.height / 4;
    let use_color = canvas.color_mode != ColorMode::Mono;
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
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
                    if canvas.pixels[idx] > BRIGHTNESS_THRESHOLD {
                        bits |= bit;
                        let (r, g, b) = canvas.colors[idx];
                        total_r += r as u32;
                        total_g += g as u32;
                        total_b += b as u32;
                        lit_count += 1;
                    }
                }
            }
            debug_assert!(bits <= 0xFF);
            let ch = char::from_u32(BRAILLE_OFFSET + bits).expect("valid braille");
            let cell = if use_color && lit_count > 0 {
                let r = (total_r / lit_count) as u8;
                let g = (total_g / lit_count) as u8;
                let b = (total_b / lit_count) as u8;
                Cell {
                    ch,
                    fg: Some(canvas.map_color(r, g, b)),
                    bg: None,
                }
            } else {
                Cell {
                    ch,
                    fg: None,
                    bg: None,
                }
            };
            cells.push(cell);
        }
    }
    CellGrid { cols, rows, cells }
}
