use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn dx(self) -> i32 {
        match self {
            Dir::Left => -1,
            Dir::Right => 1,
            _ => 0,
        }
    }

    fn dy(self) -> i32 {
        match self {
            Dir::Up => -1,
            Dir::Down => 1,
            _ => 0,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

/// Self-playing Snake game AI
pub struct Snake {
    width: usize,
    height: usize,
    body: Vec<(i32, i32)>,
    dir: Dir,
    food: (i32, i32),
    move_timer: f64,
    move_interval: f64,
    score: usize,
    game_over_timer: f64,
    rng: rand::rngs::ThreadRng,
}

impl Snake {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let grid_w = (width / 2).max(10);
        let grid_h = (height / 2).max(10);
        let cx = grid_w as i32 / 2;
        let cy = grid_h as i32 / 2;

        let mut rng = rand::rng();
        let food = (
            rng.random_range(1..grid_w as i32 - 1),
            rng.random_range(1..grid_h as i32 - 1),
        );

        Snake {
            width: grid_w,
            height: grid_h,
            body: vec![(cx, cy), (cx - 1, cy), (cx - 2, cy)],
            dir: Dir::Right,
            food,
            move_timer: 0.0,
            move_interval: 0.08,
            score: 0,
            game_over_timer: 0.0,
            rng: rand::rng(),
        }
    }

    fn reset(&mut self) {
        let cx = self.width as i32 / 2;
        let cy = self.height as i32 / 2;
        self.body = vec![(cx, cy), (cx - 1, cy), (cx - 2, cy)];
        self.dir = Dir::Right;
        self.food = (
            self.rng.random_range(1..self.width as i32 - 1),
            self.rng.random_range(1..self.height as i32 - 1),
        );
        self.score = 0;
    }

    fn ai_choose_direction(&self) -> Dir {
        let head = self.body[0];
        let fx = self.food.0;
        let fy = self.food.1;

        // Possible directions (excluding reverse)
        let dirs = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];
        let mut best_dir = self.dir;
        let mut best_dist = i32::MAX;

        for &d in &dirs {
            if d == self.dir.opposite() {
                continue;
            }

            let nx = head.0 + d.dx();
            let ny = head.1 + d.dy();

            // Check if move is safe
            if nx < 0 || nx >= self.width as i32 || ny < 0 || ny >= self.height as i32 {
                continue;
            }

            let hits_body = self.body.iter().any(|&(bx, by)| bx == nx && by == ny);
            if hits_body {
                continue;
            }

            let dist = (nx - fx).abs() + (ny - fy).abs();
            if dist < best_dist {
                best_dist = dist;
                best_dir = d;
            }
        }

        best_dir
    }

    fn spawn_food(&mut self) {
        loop {
            let fx = self.rng.random_range(1..self.width as i32 - 1);
            let fy = self.rng.random_range(1..self.height as i32 - 1);
            if !self.body.iter().any(|&(bx, by)| bx == fx && by == fy) {
                self.food = (fx, fy);
                break;
            }
            // Safety: if snake fills the grid, just place it anywhere
            if self.body.len() >= self.width * self.height - 2 {
                self.food = (fx, fy);
                break;
            }
        }
    }
}

impl Animation for Snake {
    fn name(&self) -> &str {
        "snake"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let cw = canvas.width;
        let ch = canvas.height;

        // Update grid dimensions
        self.width = (cw / 2).max(10);
        self.height = (ch / 2).max(10);

        if self.game_over_timer > 0.0 {
            self.game_over_timer -= dt;
            if self.game_over_timer <= 0.0 {
                self.reset();
            }
        } else {
            self.move_timer += dt;
            while self.move_timer >= self.move_interval {
                self.move_timer -= self.move_interval;

                // AI decides direction
                self.dir = self.ai_choose_direction();

                let head = self.body[0];
                let new_head = (head.0 + self.dir.dx(), head.1 + self.dir.dy());

                // Check collision
                let wall_hit = new_head.0 < 0
                    || new_head.0 >= self.width as i32
                    || new_head.1 < 0
                    || new_head.1 >= self.height as i32;
                let body_hit = self
                    .body
                    .iter()
                    .any(|&(bx, by)| bx == new_head.0 && by == new_head.1);

                if wall_hit || body_hit {
                    self.game_over_timer = 2.0;
                    continue;
                }

                self.body.insert(0, new_head);

                // Check food
                if new_head == self.food {
                    self.score += 1;
                    self.spawn_food();
                } else {
                    self.body.pop();
                }
            }
        }

        // Render
        canvas.clear();

        // Scale factor: map grid coords to canvas coords
        let sx = cw as f64 / self.width as f64;
        let sy = ch as f64 / self.height as f64;

        // Draw border
        for x in 0..cw {
            if x < cw {
                canvas.set_colored(x, 0, 0.3, 100, 100, 100);
            }
            if x < cw && ch > 0 {
                canvas.set_colored(x, ch - 1, 0.3, 100, 100, 100);
            }
        }
        for y in 0..ch {
            canvas.set_colored(0, y, 0.3, 100, 100, 100);
            if cw > 0 {
                canvas.set_colored(cw - 1, y, 0.3, 100, 100, 100);
            }
        }

        // Draw food
        let food_px = (self.food.0 as f64 * sx) as usize;
        let food_py = (self.food.1 as f64 * sy) as usize;
        for dy in 0..(sy as usize).max(1) {
            for dx in 0..(sx as usize).max(1) {
                let px = food_px + dx;
                let py = food_py + dy;
                if px < cw && py < ch {
                    canvas.set_colored(px, py, 1.0, 255, 50, 50);
                }
            }
        }

        // Draw snake
        let body_len = self.body.len();
        for (i, &(bx, by)) in self.body.iter().enumerate() {
            let frac = i as f64 / body_len.max(1) as f64;
            let brightness = 1.0 - frac * 0.5;

            let (r, g, b) = if i == 0 {
                (100, 255, 100) // Head
            } else if self.game_over_timer > 0.0 {
                (200, 50, 50) // Dead
            } else {
                let green = (200.0 - 100.0 * frac) as u8;
                (50, green, 50)
            };

            let base_px = (bx as f64 * sx) as usize;
            let base_py = (by as f64 * sy) as usize;
            for dy in 0..(sy as usize).max(1) {
                for dx in 0..(sx as usize).max(1) {
                    let px = base_px + dx;
                    let py = base_py + dy;
                    if px < cw && py < ch {
                        canvas.set_colored(px, py, brightness, r, g, b);
                    }
                }
            }
        }
    }
}
