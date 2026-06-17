use super::canvas::{Canvas, ColorMode};
use super::cell::{Cell, CellGrid};

/// Pixel brightness below which a half-block cell is treated as background (dark/empty).
/// Intentionally much lower than the braille threshold (0.3): half-block renders the full
/// brightness value via color scaling, so near-black pixels are visually correct as dark
/// background rather than being clipped. A low threshold preserves this detail.
const DARK_THRESHOLD: f64 = 0.02;

pub fn render(canvas: &Canvas) -> String {
    super::encoder::encode_full(&build_grid(canvas), false)
}

pub fn build_grid(canvas: &Canvas) -> CellGrid {
    let cols = canvas.width;
    let rows = canvas.height / 2;
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let top_idx = (row * 2) * canvas.width + col;
            let bot_idx = (row * 2 + 1) * canvas.width + col;
            let top_v = canvas.pixels[top_idx];
            let bot_v = canvas.pixels[bot_idx];
            let top_dark = top_v < DARK_THRESHOLD;
            let bot_dark = bot_v < DARK_THRESHOLD;
            let cell = if canvas.color_mode == ColorMode::Mono {
                let ch = match (!top_dark, !bot_dark) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                };
                Cell {
                    ch,
                    fg: None,
                    bg: None,
                }
            } else if top_dark && bot_dark {
                Cell {
                    ch: ' ',
                    fg: None,
                    bg: None,
                }
            } else {
                let (tr, tg, tb) = canvas.colors[top_idx];
                let (br, bgc, bb) = canvas.colors[bot_idx];
                let scale = |c: u8, v: f64| -> u8 { (c as f64 * v.clamp(0.0, 1.0)) as u8 };
                let top = canvas.map_color(
                    col,
                    row,
                    scale(tr, top_v),
                    scale(tg, top_v),
                    scale(tb, top_v),
                );
                let bot = canvas.map_color(
                    col,
                    row,
                    scale(br, bot_v),
                    scale(bgc, bot_v),
                    scale(bb, bot_v),
                );
                Cell {
                    ch: '▀',
                    fg: Some(top),
                    bg: Some(bot),
                }
            };
            cells.push(cell);
        }
    }
    CellGrid { cols, rows, cells }
}
