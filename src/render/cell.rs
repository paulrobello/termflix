use crossterm::style::Color;

/// One terminal cell's rendered content, mode-agnostic.
/// `fg`/`bg` of `None` mean "default/dark" (no SGR; a reset is emitted when leaving a colored cell).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

/// Row-major grid of terminal cells — the single source of truth for a frame's content.
pub struct CellGrid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>,
}

impl CellGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        CellGrid {
            cols,
            rows,
            cells: vec![
                Cell {
                    ch: ' ',
                    fg: None,
                    bg: None
                };
                cols * rows
            ],
        }
    }
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> Cell {
        self.cells[row * self.cols + col]
    }
}
