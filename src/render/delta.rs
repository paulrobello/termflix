/// Delta renderer — streams cells sequentially but skips over unchanged runs.
///
/// Instead of per-cell cursor positioning (expensive), this renders sequentially
/// like the normal renderers but detects contiguous unchanged cells and skips
/// them with a single cursor move. Best of both worlds: sequential streaming
/// for dense changes, skip-ahead for static regions.

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

/// Minimum run of unchanged cells worth skipping.
/// A cursor move costs ~8 bytes (\x1b[rr;ccH), so only skip if we save more than that.
const MIN_SKIP_RUN: usize = 4;

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
            // Prepend cursor home — renderers assume cursor starts at (1,1)
            let mut out = String::with_capacity(full_frame.len() + 4);
            out.push_str("\x1b[H");
            out.push_str(full_frame);
            return out;
        }

        // Count total changed cells to decide if delta is worth it
        let mut total_changed = 0usize;
        let total_cells = term_cols * term_rows;
        let rows_to_check = term_rows.min(current.len()).min(self.prev.len());

        for (cur_row, prev_row) in current.iter().zip(self.prev.iter()).take(rows_to_check) {
            let cols_to_check = term_cols.min(cur_row.len()).min(prev_row.len());
            for (cur, prev) in cur_row.iter().zip(prev_row.iter()).take(cols_to_check) {
                if cur != prev {
                    total_changed += 1;
                }
            }
        }

        // If most cells changed, full frame is cheaper (no cursor moves needed)
        if total_cells > 0 && total_changed * 100 / total_cells > 70 {
            self.prev = current;
            let mut out = String::with_capacity(full_frame.len() + 4);
            out.push_str("\x1b[H");
            out.push_str(full_frame);
            return out;
        }

        // Build delta output — stream sequentially, skip unchanged runs
        let mut out = String::with_capacity(total_changed * 20);
        let mut last_sgr = String::new();

        #[allow(clippy::needless_range_loop)]
        for row in 0..rows_to_check {
            let cur_row = &current[row];
            let prev_row = &self.prev[row];
            let cols_to_check = term_cols.min(cur_row.len()).min(prev_row.len());

            let mut col = 0;
            while col < cols_to_check {
                // Skip unchanged cells
                if cur_row[col] == prev_row[col] {
                    col += 1;
                    continue;
                }

                // Stream this changed cell and any subsequent changed cells
                // (or cells within a short gap of unchanged ones, to avoid cursor thrash)
                let mut cursor_placed = false;

                while col < cols_to_check {
                    let changed = cur_row[col] != prev_row[col];

                    if !changed {
                        // Look ahead: if the next few cells are also unchanged, break out
                        let mut skip_len = 0;
                        while col + skip_len < cols_to_check
                            && cur_row[col + skip_len] == prev_row[col + skip_len]
                        {
                            skip_len += 1;
                        }
                        if skip_len >= MIN_SKIP_RUN || col + skip_len >= cols_to_check {
                            // Worth skipping this run
                            break;
                        }
                        // Gap is too small — cheaper to just re-emit these cells
                    }

                    // Position cursor at start of changed segment
                    if !cursor_placed {
                        out.push_str("\x1b[");
                        out.push_str(&(row + 1).to_string());
                        out.push(';');
                        out.push_str(&(col + 1).to_string());
                        out.push('H');
                        cursor_placed = true;
                    }

                    // Emit color if changed
                    let cur = &cur_row[col];
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
                    col += 1;
                }
            }
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
                    if params == "0" || params.is_empty() {
                        current_sgr.clear();
                    } else if params == "7" {
                        current_sgr = "7".to_string();
                    } else {
                        current_sgr = params.to_string();
                    }
                }
                b'H' => {
                    if let Some(semi) = params.find(';') {
                        let r: usize = params[..semi].parse().unwrap_or(1);
                        let c: usize = params[semi + 1..].parse().unwrap_or(1);
                        row = r.saturating_sub(1);
                        col = c.saturating_sub(1);
                    } else if params.is_empty() {
                        row = 0;
                        col = 0;
                    } else {
                        let r: usize = params.parse().unwrap_or(1);
                        row = r.saturating_sub(1);
                        col = 0;
                    }
                }
                b'J' | b'h' | b'l' => {}
                _ => {}
            }
        } else {
            let remaining = &frame[i..];
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
