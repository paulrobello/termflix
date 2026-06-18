use crate::render::canvas::color_to_fg;
use crate::render::cell::{Cell, CellGrid};
use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

/// Dirty ratio at or above which a full redraw is cheaper than a scattered diff.
pub const FULL_REDRAW_THRESHOLD: f64 = 0.6;

fn color_to_bg(color: Color) -> String {
    match color {
        Color::Rgb { r, g, b } => format!("48;2;{};{};{}", r, g, b),
        Color::AnsiValue(v) => format!("48;5;{}", v),
        Color::Black => "40".into(),
        Color::DarkRed => "41".into(),
        Color::DarkGreen => "42".into(),
        Color::DarkYellow => "43".into(),
        Color::DarkBlue => "44".into(),
        Color::DarkMagenta => "45".into(),
        Color::DarkCyan => "46".into(),
        Color::Grey => "47".into(),
        Color::DarkGrey => "100".into(),
        Color::Red => "101".into(),
        Color::Green => "102".into(),
        Color::Yellow => "103".into(),
        Color::Blue => "104".into(),
        Color::Magenta => "105".into(),
        Color::Cyan => "106".into(),
        Color::White => "107".into(),
        _ => "40".into(),
    }
}

/// Emit the SGR transition to `cell`'s colors, updating tracked state. Shared by both encoders.
fn write_color_transition(
    out: &mut String,
    cell: Cell,
    last_fg: &mut Option<Color>,
    last_bg: &mut Option<Color>,
) {
    let fg_changed = cell.fg != *last_fg;
    let bg_changed = cell.bg != *last_bg;
    if !(fg_changed || bg_changed) {
        return;
    }
    match (cell.fg, cell.bg) {
        (None, None) => out.push_str("\x1b[0m"),
        (Some(f), None) => {
            out.push_str("\x1b[");
            out.push_str(&color_to_fg(f));
            out.push('m');
        }
        (None, Some(b)) => {
            out.push_str("\x1b[");
            out.push_str(&color_to_bg(b));
            out.push('m');
        }
        (Some(f), Some(b)) => {
            if fg_changed && bg_changed {
                out.push_str("\x1b[");
                out.push_str(&color_to_fg(f));
                out.push(';');
                out.push_str(&color_to_bg(b));
                out.push('m');
            } else if fg_changed {
                out.push_str("\x1b[");
                out.push_str(&color_to_fg(f));
                out.push('m');
            } else {
                out.push_str("\x1b[");
                out.push_str(&color_to_bg(b));
                out.push('m');
            }
        }
    }
    *last_fg = cell.fg;
    *last_bg = cell.bg;
}

/// Full-frame encode, byte-identical to today's per-mode `render()`.
/// `always_reset_row_end`: braille/ascii emit a row-end `\x1b[0m` unconditionally;
/// halfblock emits it only when a color is active in the row.
pub fn encode_full(grid: &CellGrid, always_reset_row_end: bool) -> String {
    let mut out = String::with_capacity(grid.cols * grid.rows * 10);
    let mut last_fg: Option<Color> = None;
    let mut last_bg: Option<Color> = None;
    for row in 0..grid.rows {
        let mut col = 0;
        while col < grid.cols {
            let cell = grid.cells[row * grid.cols + col];
            write_color_transition(&mut out, cell, &mut last_fg, &mut last_bg);
            out.push(cell.ch);
            // Advance by display width: a wide (2-column) glyph absorbs the next grid
            // cell, so skip it to keep later cells aligned to their true columns.
            col += if UnicodeWidthChar::width(cell.ch).unwrap_or(1) >= 2 {
                2
            } else {
                1
            };
        }
        let active = last_fg.is_some() || last_bg.is_some();
        if always_reset_row_end || active {
            out.push_str("\x1b[0m");
            last_fg = None;
            last_bg = None;
        }
        out.push_str("\x1b[");
        out.push_str(&(row + 2).to_string());
        out.push_str(";1H");
    }
    out
}

/// True if the grid contains any East-Asian-Wide (2-column) glyph. When set, callers should
/// use [`encode_full`] instead of [`encode_diff`], whose per-cell cursor math assumes 1 column.
pub fn grid_has_wide(grid: &CellGrid) -> bool {
    grid.cells
        .iter()
        .any(|c| UnicodeWidthChar::width(c.ch).unwrap_or(1) >= 2)
}

/// Fraction of cells that differ (0.0..=1.0). Assumes equal dimensions (caller guarantees).
pub fn dirty_ratio(prev: &CellGrid, grid: &CellGrid) -> f64 {
    let total = grid.cells.len();
    if total == 0 {
        return 0.0;
    }
    let changed = grid
        .cells
        .iter()
        .zip(prev.cells.iter())
        .filter(|(a, b)| a != b)
        .count();
    changed as f64 / total as f64
}

/// Encode only changed cells with cursor repositioning. Assumes the terminal starts at
/// default color (every prior frame ends with a reset). Ends with `\x1b[0m` if a color was emitted.
pub fn encode_diff(prev: &CellGrid, grid: &CellGrid) -> String {
    let mut out = String::new();
    let mut last_fg: Option<Color> = None;
    let mut last_bg: Option<Color> = None;
    let mut prev_col: Option<usize> = None;
    let mut prev_row: usize = 0;
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let idx = row * grid.cols + col;
            if grid.cells[idx] == prev.cells[idx] {
                continue;
            }
            let cell = grid.cells[idx];
            let adjacent = prev_col == Some(col.saturating_sub(1)) && prev_row == row && col > 0;
            if !adjacent {
                out.push_str("\x1b[");
                out.push_str(&(row + 1).to_string());
                out.push(';');
                out.push_str(&(col + 1).to_string());
                out.push('H');
            }
            write_color_transition(&mut out, cell, &mut last_fg, &mut last_bg);
            out.push(cell.ch);
            prev_col = Some(col);
            prev_row = row;
        }
    }
    if last_fg.is_some() || last_bg.is_some() {
        out.push_str("\x1b[0m");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::cell::{Cell, CellGrid};

    fn g(cells: Vec<Cell>, cols: usize, rows: usize) -> CellGrid {
        CellGrid { cols, rows, cells }
    }

    #[test]
    fn encode_full_advances_past_wide_glyph() {
        // col0 = wide katakana ア (U+30A2, 2 cols); col1 'X' is its absorbed right half and
        // must NOT be emitted; col2 'Z' must land in its true column.
        let wide = 'ア';
        assert_eq!(UnicodeWidthChar::width(wide), Some(2));
        let cells = vec![
            Cell {
                ch: wide,
                fg: None,
                bg: None,
            },
            Cell {
                ch: 'X',
                fg: None,
                bg: None,
            },
            Cell {
                ch: 'Z',
                fg: None,
                bg: None,
            },
        ];
        let grid = g(cells, 3, 1);
        let out = encode_full(&grid, false);
        assert!(out.contains('ア'), "wide glyph must be emitted: {out:?}");
        assert!(out.contains('Z'), "col-2 glyph must be emitted: {out:?}");
        assert!(
            !out.contains('X'),
            "absorbed col-1 must not be emitted: {out:?}"
        );
    }

    #[test]
    fn encode_full_narrow_grid_is_unchanged() {
        // A width-1-only grid must produce identical output to the pre-change behavior.
        let cells = vec![
            Cell {
                ch: 'a',
                fg: None,
                bg: None,
            },
            Cell {
                ch: 'b',
                fg: None,
                bg: None,
            },
            Cell {
                ch: 'c',
                fg: None,
                bg: None,
            },
        ];
        let grid = g(cells, 3, 1);
        assert_eq!(encode_full(&grid, false), "abc\x1b[2;1H");
    }

    #[test]
    fn dirty_ratio_extremes() {
        let a = g(
            vec![
                Cell {
                    ch: 'a',
                    fg: None,
                    bg: None
                };
                4
            ],
            2,
            2,
        );
        assert_eq!(dirty_ratio(&a, &a), 0.0);
        let b = g(
            vec![
                Cell {
                    ch: 'b',
                    fg: None,
                    bg: None
                };
                4
            ],
            2,
            2,
        );
        assert_eq!(dirty_ratio(&a, &b), 1.0);
    }

    #[test]
    fn encode_diff_skips_unchanged_and_moves_cursor() {
        let a = g(
            vec![
                Cell {
                    ch: 'a',
                    fg: None,
                    bg: None,
                },
                Cell {
                    ch: 'b',
                    fg: None,
                    bg: None,
                },
                Cell {
                    ch: 'c',
                    fg: None,
                    bg: None,
                },
                Cell {
                    ch: 'd',
                    fg: None,
                    bg: None,
                },
            ],
            2,
            2,
        );
        let mut bc = a.cells.clone();
        bc[1] = Cell {
            ch: 'X',
            fg: None,
            bg: None,
        };
        bc[2] = Cell {
            ch: 'Y',
            fg: None,
            bg: None,
        };
        let out = encode_diff(&a, &g(bc, 2, 2));
        assert!(
            out.contains("\x1b[1;2HX"),
            "first change needs a move: {out:?}"
        );
        assert!(
            out.contains("\x1b[2;1HY"),
            "non-adjacent change needs a move: {out:?}"
        );
        assert!(!out.contains('a') && !out.contains('c') && !out.contains('d'));
    }

    #[test]
    fn encode_diff_adjacent_needs_no_move() {
        let a = g(
            vec![
                Cell {
                    ch: 'a',
                    fg: None,
                    bg: None
                };
                3
            ],
            3,
            1,
        );
        let mut bc = a.cells.clone();
        bc[0] = Cell {
            ch: 'X',
            fg: None,
            bg: None,
        };
        bc[1] = Cell {
            ch: 'Y',
            fg: None,
            bg: None,
        };
        let out = encode_diff(&a, &g(bc, 3, 1));
        assert_eq!(
            out.matches("\x1b[").count(),
            1,
            "one escape only (the move); Y is adjacent: {out:?}"
        );
        assert!(out.contains("XY"));
    }

    // ---- Faithful fg+bg terminal simulator (for diff correctness) ----
    use crossterm::style::Color as CColor;

    fn color_rgb(c: Option<CColor>) -> Option<(u8, u8, u8)> {
        match c? {
            CColor::Rgb { r, g, b } => Some((r, g, b)),
            _ => None,
        }
    }

    #[derive(Clone, PartialEq, Debug)]
    struct TCell {
        ch: char,
        fg: Option<(u8, u8, u8)>,
        bg: Option<(u8, u8, u8)>,
    }

    struct Term {
        cells: Vec<TCell>,
        cols: usize,
        rows: usize,
        cr: usize,
        cc: usize,
        fg: Option<(u8, u8, u8)>,
        bg: Option<(u8, u8, u8)>,
    }
    impl Term {
        fn new(cols: usize, rows: usize) -> Self {
            Self {
                cells: vec![
                    TCell {
                        ch: ' ',
                        fg: None,
                        bg: None
                    };
                    cols * rows
                ],
                cols,
                rows,
                cr: 0,
                cc: 0,
                fg: None,
                bg: None,
            }
        }
        fn cell(&self, r: usize, c: usize) -> &TCell {
            &self.cells[r * self.cols + c]
        }
        fn put(&mut self, ch: char) {
            if self.cr < self.rows && self.cc < self.cols {
                let idx = self.cr * self.cols + self.cc;
                self.cells[idx] = TCell {
                    ch,
                    fg: self.fg,
                    bg: self.bg,
                };
            }
            self.cc += 1;
        }
        fn process(&mut self, data: &str) {
            let b = data.as_bytes();
            let n = b.len();
            let mut i = 0;
            while i < n {
                if b[i] == 0x1b && i + 1 < n && b[i + 1] == b'[' {
                    i += 2;
                    let s = i;
                    while i < n && (b[i].is_ascii_digit() || b[i] == b';' || b[i] == b'?') {
                        i += 1;
                    }
                    if i >= n {
                        break;
                    }
                    let params = std::str::from_utf8(&b[s..i]).unwrap_or("");
                    let cmd = b[i];
                    i += 1;
                    match cmd {
                        b'H' => {
                            let p: Vec<&str> = params.split(';').collect();
                            let r = p
                                .first()
                                .and_then(|x| x.parse::<usize>().ok())
                                .unwrap_or(1)
                                .saturating_sub(1);
                            let c = p
                                .get(1)
                                .and_then(|x| x.parse::<usize>().ok())
                                .unwrap_or(1)
                                .saturating_sub(1);
                            self.cr = r.min(self.rows.saturating_sub(1));
                            self.cc = c.min(self.cols.saturating_sub(1));
                        }
                        b'm' => {
                            if params.is_empty() || params == "0" {
                                self.fg = None;
                                self.bg = None;
                            } else {
                                let nums: Vec<u32> =
                                    params.split(';').filter_map(|x| x.parse().ok()).collect();
                                let mut k = 0;
                                while k < nums.len() {
                                    match nums[k] {
                                        0 => {
                                            self.fg = None;
                                            self.bg = None;
                                        }
                                        38 if k + 4 < nums.len() && nums[k + 1] == 2 => {
                                            self.fg = Some((
                                                nums[k + 2] as u8,
                                                nums[k + 3] as u8,
                                                nums[k + 4] as u8,
                                            ));
                                            k += 4;
                                        }
                                        48 if k + 4 < nums.len() && nums[k + 1] == 2 => {
                                            self.bg = Some((
                                                nums[k + 2] as u8,
                                                nums[k + 3] as u8,
                                                nums[k + 4] as u8,
                                            ));
                                            k += 4;
                                        }
                                        _ => {}
                                    }
                                    k += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                } else {
                    let ch = b[i];
                    let adv = if ch < 0x80 {
                        1
                    } else if ch & 0xE0 == 0xC0 {
                        2
                    } else if ch & 0xF0 == 0xE0 {
                        3
                    } else {
                        4
                    };
                    if i + adv <= n
                        && let Ok(s) = std::str::from_utf8(&b[i..i + adv])
                        && let Some(c) = s.chars().next()
                    {
                        self.put(c);
                    }
                    i += adv;
                }
            }
        }
    }

    /// Reproduces run_loop's frame delivery: full then diffs, with a status bar whose text
    /// changes periodically (mimicking the per-second FPS update). Verifies the simulated
    /// terminal matches the target grid after every frame.
    #[test]
    fn diff_with_status_bar_matches_target() {
        use crate::animations;
        use crate::render::{Canvas, ColorMode, RenderMode};
        let (cols, rows) = (50usize, 16usize);
        let display_rows = rows - 1;
        let mut canvas = Canvas::new(
            cols,
            display_rows,
            RenderMode::HalfBlock,
            ColorMode::TrueColor,
        );
        let mut anim =
            animations::create("matrix", canvas.width, canvas.height, 1.0).expect("anim");
        anim.on_resize(canvas.width, canvas.height);
        let mut term = Term::new(cols, rows);
        let mut prev: Option<CellGrid> = None;
        for f in 0..80u32 {
            anim.update(&mut canvas, 1.0 / 24.0, f as f64 / 24.0);
            canvas.apply_effects(1.0, 0.0);
            let grid = canvas.build_grid();
            let frame = match &prev {
                Some(p)
                    if p.cols == grid.cols
                        && p.rows == grid.rows
                        && dirty_ratio(p, &grid) <= FULL_REDRAW_THRESHOLD =>
                {
                    encode_diff(p, &grid)
                }
                _ => encode_full(&grid, false),
            };
            // status bar text changes every 24 frames (~1 second at 24fps)
            let fps = f / 24;
            let status = format!(
                " m | {:?} | {:?} | {} fps ",
                RenderMode::HalfBlock,
                ColorMode::TrueColor,
                fps
            );
            let padded = format!(
                "{:<width$}",
                status.chars().take(cols).collect::<String>(),
                width = cols
            );
            let mut buf = String::from("\x1b[?2026h\x1b[H");
            buf.push_str(&frame);
            buf.push_str(&format!("\x1b[{};1H\x1b[7m{}\x1b[0m", rows, padded));
            buf.push_str("\x1b[?2026l");
            term.process(&buf);
            for r in 0..grid.rows {
                for c in 0..grid.cols {
                    let tc = term.cell(r, c);
                    let gc = grid.get(r, c);
                    assert_eq!(
                        tc.ch, gc.ch,
                        "frame {f} ({r},{c}) CHAR sim {:?} grid {:?}",
                        tc.ch, gc.ch
                    );
                    assert_eq!(
                        tc.fg,
                        color_rgb(gc.fg),
                        "frame {f} ({r},{c}) FG sim {:?} grid {:?}",
                        tc.fg,
                        color_rgb(gc.fg)
                    );
                    assert_eq!(
                        tc.bg,
                        color_rgb(gc.bg),
                        "frame {f} ({r},{c}) BG sim {:?} grid {:?}",
                        tc.bg,
                        color_rgb(gc.bg)
                    );
                }
            }
            prev = Some(grid);
        }
    }

    /// Prints full-frame vs diff-frame bytes and dirty ratio per animation.
    /// Run: cargo test bench_dirty -- --ignored --nocapture
    #[test]
    #[ignore = "benchmark; run with --ignored --nocapture"]
    fn bench_dirty() {
        use crate::animations;
        use crate::render::{Canvas, ColorMode, RenderMode};
        let (cols, rows) = (120usize, 40usize);
        let n = 60u32;
        let names = ["fire", "plasma", "boids", "matrix", "nbody", "starfield"];
        println!(
            "{:<12} {:>9} {:>9} {:>8} {:>8}",
            "anim", "full_B", "diff_B", "ratio", "dirty%"
        );
        for &name in &names {
            let mut canvas = Canvas::new(cols, rows, RenderMode::HalfBlock, ColorMode::TrueColor);
            let mut anim =
                animations::create(name, canvas.width, canvas.height, 1.0).expect("anim");
            anim.on_resize(canvas.width, canvas.height);
            let mut prev: Option<CellGrid> = None;
            let (mut fb, mut db, mut ds, mut dn) = (0u64, 0u64, 0f64, 0u64);
            for i in 0..n {
                anim.update(&mut canvas, 1.0 / 24.0, i as f64 / 24.0);
                canvas.apply_effects(1.0, 0.0);
                let grid = canvas.render_cells();
                let full = encode_full(&grid, false);
                fb += full.len() as u64;
                match &prev {
                    Some(p) if p.cols == grid.cols && p.rows == grid.rows => {
                        let r = dirty_ratio(p, &grid);
                        ds += r;
                        dn += 1;
                        db += (if r > FULL_REDRAW_THRESHOLD {
                            full.len()
                        } else {
                            encode_diff(p, &grid).len()
                        }) as u64;
                    }
                    _ => db += full.len() as u64,
                }
                prev = Some(grid);
            }
            let af = fb / n as u64;
            let ad = db / n as u64;
            println!(
                "{:<12} {:>9} {:>9} {:>7.2}x {:>7.1}%",
                name,
                af,
                ad,
                ad as f64 / af.max(1) as f64,
                100.0 * ds / dn.max(1) as f64
            );
        }
    }
}
