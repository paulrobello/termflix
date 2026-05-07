use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Tetromino types with standard colors
#[derive(Clone, Copy, PartialEq)]
enum Piece {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Piece {
    fn color(self) -> (u8, u8, u8) {
        match self {
            Piece::I => (0, 255, 255), // cyan
            Piece::O => (255, 255, 0), // yellow
            Piece::T => (180, 0, 255), // purple
            Piece::S => (0, 255, 0),   // green
            Piece::Z => (255, 0, 0),   // red
            Piece::J => (0, 80, 255),  // blue
            Piece::L => (255, 165, 0), // orange
        }
    }

    fn all() -> [Piece; 7] {
        [
            Piece::I,
            Piece::O,
            Piece::T,
            Piece::S,
            Piece::Z,
            Piece::J,
            Piece::L,
        ]
    }
}

/// Rotation state for a piece: list of (col, row) offsets from origin
fn piece_cells(piece: Piece, rotation: u8) -> &'static [(i32, i32)] {
    match piece {
        Piece::I => match rotation % 4 {
            0 => &[(0, 0), (1, 0), (2, 0), (3, 0)],
            1 => &[(1, 0), (1, 1), (1, 2), (1, 3)],
            2 => &[(0, 1), (1, 1), (2, 1), (3, 1)],
            _ => &[(2, 0), (2, 1), (2, 2), (2, 3)],
        },
        Piece::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        Piece::T => match rotation % 4 {
            0 => &[(0, 0), (1, 0), (2, 0), (1, 1)],
            1 => &[(1, 0), (0, 1), (1, 1), (1, 2)],
            2 => &[(1, 0), (0, 1), (1, 1), (2, 1)],
            _ => &[(0, 0), (0, 1), (1, 1), (0, 2)],
        },
        Piece::S => match rotation % 4 {
            0 => &[(1, 0), (2, 0), (0, 1), (1, 1)],
            1 => &[(0, 0), (0, 1), (1, 1), (1, 2)],
            2 => &[(1, 0), (2, 0), (0, 1), (1, 1)],
            _ => &[(0, 0), (0, 1), (1, 1), (1, 2)],
        },
        Piece::Z => match rotation % 4 {
            0 => &[(0, 0), (1, 0), (1, 1), (2, 1)],
            1 => &[(1, 0), (0, 1), (1, 1), (0, 2)],
            2 => &[(0, 0), (1, 0), (1, 1), (2, 1)],
            _ => &[(1, 0), (0, 1), (1, 1), (0, 2)],
        },
        Piece::J => match rotation % 4 {
            0 => &[(0, 0), (0, 1), (1, 1), (2, 1)],
            1 => &[(0, 0), (1, 0), (0, 1), (0, 2)],
            2 => &[(0, 0), (1, 0), (2, 0), (2, 1)],
            _ => &[(1, 0), (1, 1), (0, 2), (1, 2)],
        },
        Piece::L => match rotation % 4 {
            0 => &[(2, 0), (0, 1), (1, 1), (2, 1)],
            1 => &[(0, 0), (0, 1), (0, 2), (1, 2)],
            2 => &[(0, 0), (1, 0), (2, 0), (0, 1)],
            _ => &[(0, 0), (1, 0), (1, 1), (1, 2)],
        },
    }
}

/// Active falling piece
#[derive(Clone)]
struct ActivePiece {
    piece: Piece,
    rotation: u8,
    col: i32,
    row: i32,
}

impl ActivePiece {
    fn cells(&self) -> &'static [(i32, i32)] {
        piece_cells(self.piece, self.rotation)
    }

    #[allow(dead_code)]
    fn abs_cells(&self) -> Vec<(i32, i32)> {
        self.cells()
            .iter()
            .map(|&(cx, cy)| (self.col + cx, self.row + cy))
            .collect()
    }
}

/// Best placement found by AI
#[derive(Clone)]
struct Placement {
    col: i32,
    rotation: u8,
}

/// Self-playing Tetris AI
pub struct Tetris {
    board_cols: usize,
    board_rows: usize,
    board_x: usize,
    board_y: usize,
    board: Vec<Option<Piece>>,
    current: Option<ActivePiece>,
    next_piece: Piece,
    #[allow(dead_code)]
    move_timer: f64,
    move_interval: f64,
    drop_timer: f64,
    drop_interval: f64,
    score: u32,
    lines_cleared: u32,
    line_flash_timer: f64,
    flashing_rows: Vec<usize>,
    ai_target: Option<Placement>,
    ai_move_timer: f64,
    game_over_timer: f64,
    rng: rand::rngs::ThreadRng,
}

impl Tetris {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let board_cols = 12;
        let board_rows = (height * 2 / 3).clamp(12, 30);
        let board_x = if width > board_cols + 12 {
            (width - board_cols - 10) / 2
        } else {
            2
        };
        let board_y = if height > board_rows + 4 {
            (height - board_rows) / 2
        } else {
            2
        };

        let mut rng = rand::rng();
        let next_piece = rng.random_piece();

        let mut tetris = Tetris {
            board_cols,
            board_rows,
            board_x,
            board_y,
            board: vec![None; board_cols * board_rows],
            current: None,
            next_piece,
            move_timer: 0.0,
            move_interval: 0.04,
            drop_timer: 0.0,
            drop_interval: 0.15,
            score: 0,
            lines_cleared: 0,
            line_flash_timer: 0.0,
            flashing_rows: Vec::new(),
            ai_target: None,
            ai_move_timer: 0.0,
            game_over_timer: 0.0,
            rng,
        };
        tetris.spawn_piece();
        tetris
    }

    fn reset(&mut self) {
        self.board.fill(None);
        self.score = 0;
        self.lines_cleared = 0;
        self.line_flash_timer = 0.0;
        self.flashing_rows.clear();
        self.ai_target = None;
        self.next_piece = self.rng.random_piece();
        self.spawn_piece();
    }

    fn spawn_piece(&mut self) {
        let piece = self.next_piece;
        self.next_piece = self.rng.random_piece();

        let cells = piece_cells(piece, 0);
        let max_c = cells.iter().map(|&(c, _)| c).max().unwrap_or(0);
        let col = (self.board_cols as i32 - max_c - 1) / 2;
        let row = 0;

        let ap = ActivePiece {
            piece,
            rotation: 0,
            col,
            row,
        };

        // Check if spawn position is blocked (game over)
        if !self.piece_fits(&ap) {
            self.current = None;
            self.game_over_timer = 2.5;
            return;
        }

        self.current = Some(ap);
        self.drop_timer = 0.0;
        self.ai_target = None;
        self.compute_ai_target();
    }

    fn piece_fits(&self, piece: &ActivePiece) -> bool {
        for &(cx, cy) in piece.cells() {
            let bx = piece.col + cx;
            let by = piece.row + cy;
            if bx < 0 || bx >= self.board_cols as i32 || by < 0 || by >= self.board_rows as i32 {
                return false;
            }
            if self.board[by as usize * self.board_cols + bx as usize].is_some() {
                return false;
            }
        }
        true
    }

    #[allow(dead_code)]
    fn board_get(&self, col: i32, row: i32) -> Option<Piece> {
        if col < 0 || col >= self.board_cols as i32 || row < 0 || row >= self.board_rows as i32 {
            return Some(Piece::I); // treat out-of-bounds as filled
        }
        self.board[row as usize * self.board_cols + col as usize]
    }

    fn board_set(&mut self, col: i32, row: i32, piece: Piece) {
        if col >= 0
            && (col as usize) < self.board_cols
            && row >= 0
            && (row as usize) < self.board_rows
        {
            self.board[row as usize * self.board_cols + col as usize] = Some(piece);
        }
    }

    fn lock_piece(&mut self) {
        let cells: Vec<(i32, i32, Piece)> = match self.current {
            Some(ref ap) => ap
                .cells()
                .iter()
                .map(|&(cx, cy)| (ap.col + cx, ap.row + cy, ap.piece))
                .collect(),
            None => return,
        };
        for (x, y, piece) in cells {
            self.board_set(x, y, piece);
        }
        self.check_lines();
    }

    fn check_lines(&mut self) {
        let mut full_rows = Vec::new();
        for row in 0..self.board_rows {
            let row_full =
                (0..self.board_cols).all(|col| self.board[row * self.board_cols + col].is_some());
            if row_full {
                full_rows.push(row);
            }
        }

        if full_rows.is_empty() {
            self.spawn_piece();
        } else {
            self.flashing_rows = full_rows;
            self.line_flash_timer = 0.25;
            let n = self.flashing_rows.len() as u32;
            self.score += match n {
                1 => 100,
                2 => 300,
                3 => 500,
                _ => 800,
            };
            self.lines_cleared += n;
            // Speed up slightly as lines are cleared
            self.drop_interval = (0.15 - self.lines_cleared as f64 * 0.002).max(0.05);
        }
    }

    fn remove_flashing_rows(&mut self) {
        // Sort rows descending so removal doesn't shift indices
        self.flashing_rows.sort_by(|a, b| b.cmp(a));
        for &row in &self.flashing_rows {
            // Shift everything above down
            for r in (1..=row).rev() {
                for col in 0..self.board_cols {
                    self.board[r * self.board_cols + col] =
                        self.board[(r - 1) * self.board_cols + col];
                }
            }
            // Clear top row
            for col in 0..self.board_cols {
                self.board[col] = None;
            }
        }
        self.flashing_rows.clear();
        self.spawn_piece();
    }

    fn ghost_row(&self, ap: &ActivePiece) -> i32 {
        let mut ghost = ap.clone();
        loop {
            ghost.row += 1;
            if !self.piece_fits(&ghost) {
                ghost.row -= 1;
                return ghost.row;
            }
        }
    }

    fn compute_ai_target(&mut self) {
        let ap = match self.current {
            Some(ref ap) => ap,
            None => return,
        };

        let piece = ap.piece;
        let num_rotations = if piece == Piece::O { 1 } else { 4 };

        let mut best_score = f64::NEG_INFINITY;
        let mut best_placement = Placement {
            col: ap.col,
            rotation: 0,
        };

        for rot in 0..num_rotations {
            let test = ActivePiece {
                piece,
                rotation: rot,
                col: 0,
                row: 0,
            };
            let cells = test.cells();
            let min_c = cells.iter().map(|&(c, _)| c).min().unwrap_or(0);
            let max_c = cells.iter().map(|&(c, _)| c).max().unwrap_or(0);

            for col in -min_c..=(self.board_cols as i32 - 1 - max_c) {
                let mut candidate = ActivePiece {
                    piece,
                    rotation: rot,
                    col,
                    row: 0,
                };

                // Drop to bottom
                loop {
                    candidate.row += 1;
                    if !self.piece_fits(&candidate) {
                        candidate.row -= 1;
                        break;
                    }
                }

                if candidate.row < 0 {
                    continue;
                }

                // Evaluate this placement
                let eval = self.evaluate_placement(&candidate);
                if eval > best_score {
                    best_score = eval;
                    best_placement = Placement { col, rotation: rot };
                }
            }
        }

        self.ai_target = Some(best_placement);
    }

    fn evaluate_placement(&self, ap: &ActivePiece) -> f64 {
        // Place piece on a temporary board copy
        let mut temp_board = self.board.clone();
        for &(cx, cy) in ap.cells() {
            let bx = ap.col + cx;
            let by = ap.row + cy;
            if bx >= 0
                && (bx as usize) < self.board_cols
                && by >= 0
                && (by as usize) < self.board_rows
            {
                temp_board[by as usize * self.board_cols + bx as usize] = Some(ap.piece);
            }
        }

        // Count complete lines (reward)
        let mut complete_lines = 0;
        for row in 0..self.board_rows {
            if (0..self.board_cols).all(|col| temp_board[row * self.board_cols + col].is_some()) {
                complete_lines += 1;
            }
        }

        // Count aggregate height (penalty)
        let mut aggregate_height = 0;
        for col in 0..self.board_cols {
            for row in 0..self.board_rows {
                if temp_board[row * self.board_cols + col].is_some() {
                    aggregate_height += self.board_rows - row;
                    break;
                }
            }
        }

        // Count holes (penalty)
        let mut holes = 0;
        for col in 0..self.board_cols {
            let mut found_block = false;
            for row in 0..self.board_rows {
                if temp_board[row * self.board_cols + col].is_some() {
                    found_block = true;
                } else if found_block {
                    holes += 1;
                }
            }
        }

        // Count bumpiness (penalty)
        let mut heights = Vec::with_capacity(self.board_cols);
        for col in 0..self.board_cols {
            let mut h = 0;
            for row in 0..self.board_rows {
                if temp_board[row * self.board_cols + col].is_some() {
                    h = self.board_rows - row;
                    break;
                }
            }
            heights.push(h);
        }
        let mut bumpiness = 0;
        for i in 1..heights.len() {
            bumpiness += (heights[i] as i32 - heights[i - 1] as i32).unsigned_abs();
        }

        // Weighted scoring
        (complete_lines as f64 * 760.0)
            - (aggregate_height as f64 * 510.0)
            - (holes as f64 * 3566.0)
            - (bumpiness as f64 * 184.0)
    }

    fn ai_step(&mut self) {
        let target = match self.ai_target {
            Some(ref t) => t.clone(),
            None => return,
        };

        let ap = match self.current {
            Some(ref ap) => ap.clone(),
            None => return,
        };

        // Rotate first if needed
        if ap.rotation != target.rotation {
            let mut rotated = ap.clone();
            rotated.rotation = (rotated.rotation + 1) % 4;
            if self.piece_fits(&rotated) {
                self.current = Some(rotated);
                return;
            }
            // Try wall kick: try offsets -1, +1, -2, +2
            for offset in &[-1i32, 1, -2, 2] {
                let mut kicked = rotated.clone();
                kicked.col += offset;
                if self.piece_fits(&kicked) {
                    self.current = Some(kicked);
                    return;
                }
            }
            // Can't rotate yet, try moving instead
        }

        // Move horizontally toward target
        let ap = match self.current {
            Some(ref ap) => ap.clone(),
            None => return,
        };

        if ap.col < target.col {
            let mut moved = ap.clone();
            moved.col += 1;
            if self.piece_fits(&moved) {
                self.current = Some(moved);
            }
        } else if ap.col > target.col {
            let mut moved = ap.clone();
            moved.col -= 1;
            if self.piece_fits(&moved) {
                self.current = Some(moved);
            }
        }
    }
}

trait RandomPiece {
    fn random_piece(&mut self) -> Piece;
}

impl RandomPiece for rand::rngs::ThreadRng {
    fn random_piece(&mut self) -> Piece {
        let pieces = Piece::all();
        let idx = self.random_range(0usize..7);
        pieces[idx]
    }
}

impl Animation for Tetris {
    fn name(&self) -> &str {
        "tetris"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.board_cols = 12;
        self.board_rows = (height * 2 / 3).clamp(12, 30);
        self.board_x = if width > self.board_cols + 12 {
            (width - self.board_cols - 10) / 2
        } else {
            2
        };
        self.board_y = if height > self.board_rows + 4 {
            (height - self.board_rows) / 2
        } else {
            2
        };
        self.board = vec![None; self.board_cols * self.board_rows];
        self.reset();
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Recalculate board position based on current canvas size
        let cw = canvas.width;
        let ch = canvas.height;
        self.board_x = if cw > self.board_cols + 12 {
            (cw - self.board_cols - 10) / 2
        } else {
            2
        };
        self.board_y = if ch > self.board_rows + 4 {
            (ch - self.board_rows) / 2
        } else {
            2
        };

        // Game over pause
        if self.game_over_timer > 0.0 {
            self.game_over_timer -= dt;
            if self.game_over_timer <= 0.0 {
                self.reset();
            }
            self.render(canvas);
            return;
        }

        // Line clear flash
        if self.line_flash_timer > 0.0 {
            self.line_flash_timer -= dt;
            if self.line_flash_timer <= 0.0 {
                self.remove_flashing_rows();
            }
            self.render(canvas);
            return;
        }

        // AI moves (rotation + lateral)
        self.ai_move_timer += dt;
        if self.ai_move_timer >= self.move_interval {
            self.ai_move_timer -= self.move_interval;
            self.ai_step();
        }

        // Gravity drop
        self.drop_timer += dt;
        if self.drop_timer >= self.drop_interval {
            self.drop_timer -= self.drop_interval;

            if let Some(ref ap) = self.current {
                let mut dropped = ap.clone();
                dropped.row += 1;
                if self.piece_fits(&dropped) {
                    self.current = Some(dropped);
                } else {
                    // Lock
                    self.lock_piece();
                }
            }
        }

        self.render(canvas);
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::HalfBlock
    }
}

impl Tetris {
    fn render(&self, canvas: &mut Canvas) {
        canvas.clear();

        let bx = self.board_x;
        let by = self.board_y;
        let bw = self.board_cols;
        let bh = self.board_rows;

        // Draw board border
        let border_color: (u8, u8, u8) = (60, 60, 80);
        for row in 0..bh {
            // Left border
            let px = bx;
            let py = by + row;
            if px > 0 && py < canvas.height {
                canvas.set_colored(
                    px - 1,
                    py,
                    0.5,
                    border_color.0,
                    border_color.1,
                    border_color.2,
                );
            }
            // Right border
            let px = bx + bw;
            if px < canvas.width && py < canvas.height {
                canvas.set_colored(px, py, 0.5, border_color.0, border_color.1, border_color.2);
            }
        }
        // Bottom border
        for col in 0..=bw {
            let px = bx + col;
            let py = by + bh;
            if px < canvas.width && py < canvas.height {
                canvas.set_colored(px, py, 0.5, border_color.0, border_color.1, border_color.2);
            }
        }

        // Draw board contents
        for row in 0..bh {
            for col in 0..bw {
                let px = bx + col;
                let py = by + row;
                if px >= canvas.width || py >= canvas.height {
                    continue;
                }

                // Check if this row is flashing
                let is_flashing = self.flashing_rows.contains(&row);

                if let Some(piece) = self.board[row * bw + col] {
                    if is_flashing {
                        // Flash white
                        canvas.set_colored(px, py, 1.0, 255, 255, 255);
                    } else {
                        let (r, g, b) = piece.color();
                        canvas.set_colored(px, py, 0.9, r, g, b);
                    }
                } else if row == 0 {
                    // Faint top danger zone
                    canvas.set_colored(px, py, 0.05, 40, 0, 0);
                }
            }
        }

        // Draw ghost piece
        if let Some(ref ap) = self.current
            && self.game_over_timer <= 0.0
        {
            let ghost_row = self.ghost_row(ap);
            for &(cx, cy) in ap.cells() {
                let px = bx + (ap.col + cx) as usize;
                let py = by + (ghost_row + cy) as usize;
                if px < canvas.width && py < canvas.height {
                    let (r, g, b) = ap.piece.color();
                    canvas.set_colored(px, py, 0.2, r / 3, g / 3, b / 3);
                }
            }
        }

        // Draw current piece
        if let Some(ref ap) = self.current {
            for &(cx, cy) in ap.cells() {
                let px = bx + (ap.col + cx) as usize;
                let py = by + (ap.row + cy) as usize;
                if px < canvas.width && py < canvas.height {
                    if self.game_over_timer > 0.0 {
                        canvas.set_colored(px, py, 1.0, 255, 60, 60);
                    } else {
                        let (r, g, b) = ap.piece.color();
                        canvas.set_colored(px, py, 1.0, r, g, b);
                    }
                }
            }
        }

        // HUD area (right of the board)
        let hud_x = bx + bw + 3;
        let mut hud_y = by;

        // Draw "NEXT" label
        if hud_x + 6 < canvas.width && hud_y < canvas.height {
            for (i, ch) in "NEXT".chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 200, 200, 200);
            }
        }
        hud_y += 2;

        // Draw next piece preview
        let preview_cells = piece_cells(self.next_piece, 0);
        for &(cx, cy) in preview_cells {
            let px = hud_x + cx as usize;
            let py = hud_y + cy as usize;
            if px < canvas.width && py < canvas.height {
                let (r, g, b) = self.next_piece.color();
                canvas.set_colored(px, py, 0.8, r, g, b);
            }
        }

        hud_y += 5;

        // Score
        if hud_x < canvas.width && hud_y < canvas.height {
            for (i, ch) in "SCORE".chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 180, 180, 180);
            }
        }
        hud_y += 1;
        let score_str = format!("{}", self.score);
        if hud_y < canvas.height {
            for (i, ch) in score_str.chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 255, 255, 100);
            }
        }
        hud_y += 2;

        // Lines
        if hud_x < canvas.width && hud_y < canvas.height {
            for (i, ch) in "LINES".chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 180, 180, 180);
            }
        }
        hud_y += 1;
        let lines_str = format!("{}", self.lines_cleared);
        if hud_y < canvas.height {
            for (i, ch) in lines_str.chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 100, 255, 100);
            }
        }
        hud_y += 2;

        // Level (derived from lines)
        let level = self.lines_cleared / 10 + 1;
        if hud_x < canvas.width && hud_y < canvas.height {
            for (i, ch) in "LEVEL".chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 180, 180, 180);
            }
        }
        hud_y += 1;
        let level_str = format!("{}", level);
        if hud_y < canvas.height {
            for (i, ch) in level_str.chars().enumerate() {
                canvas.set_char(hud_x + i, hud_y, ch, 100, 200, 255);
            }
        }

        // Game over text
        if self.game_over_timer > 0.0 {
            let go_text = "GAME OVER";
            let go_x = bx + (bw / 2).saturating_sub(go_text.len() / 2);
            let go_y = by + bh / 2;
            if go_y < canvas.height {
                for (i, ch) in go_text.chars().enumerate() {
                    let px = go_x + i;
                    if px < canvas.width {
                        canvas.set_char(px, go_y, ch, 255, 50, 50);
                    }
                }
            }
        }
    }
}
