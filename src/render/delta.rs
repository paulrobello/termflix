/// Delta renderer — only emits escape sequences for cells that changed since last frame.
/// Works with any render mode by parsing the rendered output into a cell grid,
/// then diffing against the previous frame.

#[derive(Clone, PartialEq)]
struct Cell {
    ch: char,
    /// The raw SGR parameters (e.g. "38;2;255;128;0" or "38;2;255;128;0;48;2;0;0;0")
    sgr: String,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            sgr: String::new(),
        }
    }
}

pub struct DeltaRenderer {
    prev: Vec<Vec<Cell>>,
    term_cols: usize,
    term_rows: usize,
    /// Force full redraw on next frame
    force_full: bool,
}

impl DeltaRenderer {
    pub fn new() -> Self {
        DeltaRenderer {
            prev: Vec::new(),
            term_cols: 0,
            term_rows: 0,
            force_full: true,
        }
    }

    /// Call when terminal resizes or render mode changes
    pub fn invalidate(&mut self) {
        self.force_full = true;
    }

    /// Takes raw rendered frame string from canvas.render() and produces
    /// a delta-optimized output string. Falls back to full render on first
    /// frame or after invalidation.
    pub fn render_delta(&mut self, full_frame: &str, term_cols: usize, term_rows: usize) -> String {
        // Parse current frame into cell grid
        let current = parse_frame(full_frame, term_cols, term_rows);

        // Size changed — force full redraw
        if term_cols != self.term_cols || term_rows != self.term_rows {
            self.term_cols = term_cols;
            self.term_rows = term_rows;
            self.force_full = true;
        }

        if self.force_full || self.prev.is_empty() {
            self.prev = current;
            self.force_full = false;
            // Return original frame as-is
            return full_frame.to_string();
        }

        // Build delta output
        let mut out = String::with_capacity(term_cols * term_rows * 4);
        let mut last_sgr = String::new();
        let mut changed = 0u32;
        let total = (term_cols * term_rows) as u32;

        for (row, (cur_row, prev_row)) in current
            .iter()
            .zip(self.prev.iter())
            .take(term_rows)
            .enumerate()
        {
            for (col, (cur, prev)) in cur_row
                .iter()
                .zip(prev_row.iter())
                .take(term_cols)
                .enumerate()
            {
                if cur != prev {
                    changed += 1;
                    // Move cursor to position (1-indexed)
                    out.push_str("\x1b[");
                    // row+1 for 1-indexed, +1 for the \x1b[H offset used by renderers
                    let screen_row = row + 1;
                    out.push_str(&screen_row.to_string());
                    out.push(';');
                    out.push_str(&(col + 1).to_string());
                    out.push('H');

                    // Set color if different from last emitted
                    if cur.sgr != last_sgr {
                        if cur.sgr.is_empty() {
                            out.push_str("\x1b[0m");
                        } else {
                            out.push_str("\x1b[");
                            out.push_str(&cur.sgr);
                            out.push('m');
                        }
                        last_sgr.clone_from(&cur.sgr);
                    }
                    out.push(cur.ch);
                }
            }
        }

        // If more than 60% of cells changed, full frame is likely smaller
        if changed > total * 3 / 5 {
            self.prev = current;
            return full_frame.to_string();
        }

        // Reset colors at end
        if !last_sgr.is_empty() {
            out.push_str("\x1b[0m");
        }

        self.prev = current;
        out
    }
}

/// Parse a rendered frame string into a grid of cells.
/// Handles ANSI escape sequences (SGR only) and cursor movement.
fn parse_frame(frame: &str, cols: usize, rows: usize) -> Vec<Vec<Cell>> {
    let mut grid: Vec<Vec<Cell>> = (0..rows)
        .map(|_| (0..cols).map(|_| Cell::default()).collect())
        .collect();

    let mut row: usize = 0;
    let mut col: usize = 0;
    let mut current_sgr = String::new();

    let bytes = frame.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            // Parse CSI sequence
            i += 2;
            let start = i;
            while i < len
                && bytes[i] != b'm'
                && bytes[i] != b'H'
                && bytes[i] != b'J'
                && bytes[i] != b'h'
                && bytes[i] != b'l'
            {
                i += 1;
            }
            if i >= len {
                break;
            }
            let params = &frame[start..i];
            let terminator = bytes[i];
            i += 1;

            match terminator {
                b'm' => {
                    // SGR (color) sequence
                    if params == "0" || params.is_empty() {
                        current_sgr.clear();
                    } else if params == "7" {
                        // Reverse video (status bar) — treat as special SGR
                        current_sgr = "7".to_string();
                    } else {
                        current_sgr = params.to_string();
                    }
                }
                b'H' => {
                    // Cursor position: row;colH
                    if let Some(semi) = params.find(';') {
                        let r: usize = params[..semi].parse().unwrap_or(1);
                        let c: usize = params[semi + 1..].parse().unwrap_or(1);
                        row = r.saturating_sub(1);
                        col = c.saturating_sub(1);
                    } else if params.is_empty() {
                        row = 0;
                        col = 0;
                    } else {
                        // Just row, col=0 — shouldn't happen often
                        let r: usize = params.parse().unwrap_or(1);
                        row = r.saturating_sub(1);
                        col = 0;
                    }
                }
                b'J' => {
                    // Clear screen — ignore for parsing
                }
                b'h' | b'l' => {
                    // Private mode set/reset (synchronized output) — ignore
                }
                _ => {}
            }
        } else {
            // Regular character — could be multi-byte UTF-8
            let ch_start = i;
            // Decode one UTF-8 character
            let remaining = &frame[ch_start..];
            if let Some(ch) = remaining.chars().next() {
                if row < rows && col < cols {
                    grid[row][col] = Cell {
                        ch,
                        sgr: current_sgr.clone(),
                    };
                }
                col += 1;
                i += ch.len_utf8();
            } else {
                i += 1;
            }
        }
    }

    grid
}
