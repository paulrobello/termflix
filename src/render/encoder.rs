use crate::render::canvas::color_to_fg;
use crate::render::cell::{Cell, CellGrid};
use crossterm::style::Color;

/// Dirty ratio at or above which a full redraw is cheaper than a scattered diff.
#[allow(dead_code)] // wired in Task 4 (render loop chooses full vs diff)
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
        for col in 0..grid.cols {
            let cell = grid.cells[row * grid.cols + col];
            write_color_transition(&mut out, cell, &mut last_fg, &mut last_bg);
            out.push(cell.ch);
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

/// Fraction of cells that differ (0.0..=1.0). Assumes equal dimensions (caller guarantees).
#[allow(dead_code)] // wired in Task 4 (render loop chooses full vs diff)
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
#[allow(dead_code)] // wired in Task 4 (render loop chooses full vs diff)
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
}
