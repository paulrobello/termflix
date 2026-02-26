use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Conway's Game of Life at sub-cell resolution
pub struct GameOfLife {
    width: usize,
    height: usize,
    cells: Vec<bool>,
    generation: u64,
    accumulator: f64,
    step_interval: f64,
    // Stagnation detection: track population over time
    prev_pop: usize,
    stable_count: u32,
    // Track previous state hash for oscillator detection
    prev_hash: u64,
    hash_stable_count: u32,
    rng: rand::rngs::ThreadRng,
}

impl GameOfLife {
    pub fn new(width: usize, height: usize) -> Self {
        let mut rng = rand::rng();
        let size = width * height;
        let density = rng.random_range(0.2..0.5); // vary initial density
        let cells: Vec<bool> = (0..size)
            .map(|_| rng.random_range(0.0..1.0) > (1.0 - density))
            .collect();
        let pop = cells.iter().filter(|&&c| c).count();
        GameOfLife {
            width,
            height,
            cells,
            generation: 0,
            accumulator: 0.0,
            step_interval: 0.08,
            prev_pop: pop,
            stable_count: 0,
            prev_hash: 0,
            hash_stable_count: 0,
            rng: rand::rng(),
        }
    }

    fn step(&mut self) {
        let mut next = vec![false; self.width * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                let neighbors = self.count_neighbors(x, y);
                let alive = self.cells[y * self.width + x];
                next[y * self.width + x] =
                    matches!((alive, neighbors), (true, 2) | (true, 3) | (false, 3));
            }
        }
        self.cells = next;
        self.generation += 1;

        // Check for stagnation via population stability
        let pop = self.cells.iter().filter(|&&c| c).count();
        if pop == self.prev_pop || pop == 0 {
            self.stable_count += 1;
        } else {
            self.stable_count = 0;
        }
        self.prev_pop = pop;

        // Check for oscillators via simple hash
        let hash = self.cell_hash();
        if hash == self.prev_hash {
            self.hash_stable_count += 1;
        } else {
            self.hash_stable_count = 0;
        }
        self.prev_hash = hash;

        // Full reset if stagnant (population unchanged for 60 steps, or oscillator, or dead)
        if self.stable_count > 60 || self.hash_stable_count > 10 || pop == 0 {
            *self = GameOfLife::new(self.width, self.height);
        }
        // Inject chaos periodically to keep things interesting
        else if self.generation.is_multiple_of(300) {
            // Spawn a random pattern (glider gun, r-pentomino, etc)
            let cx = self.rng.random_range(10..self.width.saturating_sub(10).max(11));
            let cy = self.rng.random_range(10..self.height.saturating_sub(10).max(11));
            // R-pentomino — classic long-lived pattern
            let pattern = [(0, 0), (1, 0), (-1, 1), (0, 1), (0, 2)];
            for (dx, dy) in pattern {
                let x = (cx as i32 + dx).rem_euclid(self.width as i32) as usize;
                let y = (cy as i32 + dy).rem_euclid(self.height as i32) as usize;
                self.cells[y * self.width + x] = true;
            }
        }
    }

    fn cell_hash(&self) -> u64 {
        // Just use population as cheap "hash" — combined with prev_pop check catches most cases
        self.prev_pop as u64
    }

    fn count_neighbors(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for dy in [-1i32, 0, 1] {
            for dx in [-1i32, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (x as i32 + dx).rem_euclid(self.width as i32) as usize;
                let ny = (y as i32 + dy).rem_euclid(self.height as i32) as usize;
                if self.cells[ny * self.width + nx] {
                    count += 1;
                }
            }
        }
        count
    }
}

impl Animation for GameOfLife {
    fn name(&self) -> &str {
        "life"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        self.accumulator += dt;
        while self.accumulator >= self.step_interval {
            self.step();
            self.accumulator -= self.step_interval;
        }

        if self.width != canvas.width || self.height != canvas.height {
            *self = GameOfLife::new(canvas.width, canvas.height);
        }

        canvas.clear();
        for y in 0..self.height.min(canvas.height) {
            let row = y * self.width;
            for x in 0..self.width.min(canvas.width) {
                if self.cells[row + x] {
                    canvas.set_colored(x, y, 1.0, 50, 255, 50);
                }
            }
        }
    }
}
