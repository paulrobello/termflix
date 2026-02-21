use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    fn turn_right(self) -> Self {
        match self {
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
        }
    }

    fn turn_left(self) -> Self {
        match self {
            Direction::Up => Direction::Left,
            Direction::Left => Direction::Down,
            Direction::Down => Direction::Right,
            Direction::Right => Direction::Up,
        }
    }

    fn dx(self) -> i32 {
        match self {
            Direction::Left => -1,
            Direction::Right => 1,
            _ => 0,
        }
    }

    fn dy(self) -> i32 {
        match self {
            Direction::Up => -1,
            Direction::Down => 1,
            _ => 0,
        }
    }
}

/// Langton's Ant cellular automaton showing emergent highway
pub struct Langton {
    width: usize,
    height: usize,
    grid: Vec<bool>,
    ant_x: i32,
    ant_y: i32,
    ant_dir: Direction,
    steps: usize,
    steps_per_frame: usize,
    total_steps: usize,
}

impl Langton {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let grid = vec![false; width * height];
        let dir = match rng.random_range(0u8..4) {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            _ => Direction::Left,
        };
        // Randomize start position within central region
        let ant_x = rng.random_range(width as i32 / 3..width as i32 * 2 / 3);
        let ant_y = rng.random_range(height as i32 / 3..height as i32 * 2 / 3);
        Langton {
            width,
            height,
            grid,
            ant_x,
            ant_y,
            ant_dir: dir,
            steps: 0,
            steps_per_frame: (100.0 * scale) as usize,
            total_steps: 0,
        }
    }

    fn reset(&mut self) {
        let mut rng = rand::rng();
        self.grid = vec![false; self.width * self.height];
        self.ant_x = rng.random_range(self.width as i32 / 3..self.width as i32 * 2 / 3);
        self.ant_y = rng.random_range(self.height as i32 / 3..self.height as i32 * 2 / 3);
        self.ant_dir = match rng.random_range(0u8..4) {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            _ => Direction::Left,
        };
        self.steps = 0;
        self.total_steps = 0;
    }
}

impl Animation for Langton {
    fn name(&self) -> &str {
        "langton"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, _time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;

        // Resize grid if needed
        if self.grid.len() != self.width * self.height {
            self.reset();
        }

        // Reset if ant has been going for a very long time
        if self.total_steps > self.width * self.height * 3 {
            self.reset();
        }

        // Simulate steps
        for _ in 0..self.steps_per_frame {
            let ax = self.ant_x as usize;
            let ay = self.ant_y as usize;

            if ax < self.width && ay < self.height {
                let idx = ay * self.width + ax;
                let cell = self.grid[idx];

                if cell {
                    // On black: turn left, flip to white
                    self.ant_dir = self.ant_dir.turn_left();
                    self.grid[idx] = false;
                } else {
                    // On white: turn right, flip to black
                    self.ant_dir = self.ant_dir.turn_right();
                    self.grid[idx] = true;
                }
            }

            // Move forward
            self.ant_x += self.ant_dir.dx();
            self.ant_y += self.ant_dir.dy();

            // Wrap around
            if self.ant_x < 0 {
                self.ant_x += self.width as i32;
            }
            if self.ant_x >= self.width as i32 {
                self.ant_x -= self.width as i32;
            }
            if self.ant_y < 0 {
                self.ant_y += self.height as i32;
            }
            if self.ant_y >= self.height as i32 {
                self.ant_y -= self.height as i32;
            }

            self.steps += 1;
            self.total_steps += 1;
        }

        // Render
        canvas.clear();
        for y in 0..self.height.min(canvas.height) {
            for x in 0..self.width.min(canvas.width) {
                if self.grid[y * self.width + x] {
                    // Colored based on position for visual interest
                    let hue =
                        ((x as f64 / self.width as f64) + (y as f64 / self.height as f64)) * 0.5;
                    let (r, g, b) = hsv_to_rgb(hue, 0.7, 0.8);
                    canvas.set_colored(x, y, 0.7, r, g, b);
                }
            }
        }

        // Draw ant
        let ax = self.ant_x as usize;
        let ay = self.ant_y as usize;
        if ax < canvas.width && ay < canvas.height {
            canvas.set_colored(ax, ay, 1.0, 255, 50, 50);
        }
        // Ant glow
        for &(ox, oy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let gx = (self.ant_x + ox) as usize;
            let gy = (self.ant_y + oy) as usize;
            if gx < canvas.width && gy < canvas.height {
                canvas.set_colored(gx, gy, 0.5, 255, 100, 50);
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
