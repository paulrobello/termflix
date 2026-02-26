use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Diffusion-limited aggregation crystal growth from center seed
pub struct Crystallize {
    width: usize,
    height: usize,
    grid: Vec<u8>,
    walkers: Vec<(f64, f64)>,
    growth_timer: f64,
    steps_per_frame: usize,
    color_cycle: f64,
    rng: rand::rngs::ThreadRng,
}

impl Crystallize {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut grid = vec![0u8; width * height];
        // Place seed at center
        let cx = width / 2;
        let cy = height / 2;
        if cx < width && cy < height {
            grid[cy * width + cx] = 1;
        }

        let walker_count = (200.0 * scale) as usize;
        let mut rng = rand::rng();
        let walkers = (0..walker_count)
            .map(|_| {
                let angle = rng.random_range(0.0..std::f64::consts::TAU);
                let dist = rng.random_range(10.0..(width.min(height) as f64 * 0.4));
                let x = width as f64 * 0.5 + angle.cos() * dist;
                let y = height as f64 * 0.5 + angle.sin() * dist;
                (x, y)
            })
            .collect();

        Crystallize {
            width,
            height,
            grid,
            walkers,
            growth_timer: 0.0,
            steps_per_frame: (50.0 * scale) as usize,
            color_cycle: 0.0,
            rng: rand::rng(),
        }
    }
}

impl Crystallize {
    #[allow(dead_code)]
    fn has_neighbor(&self, x: usize, y: usize) -> bool {
        let w = self.width;
        let h = self.height;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && nx < w as i32
                    && ny >= 0
                    && ny < h as i32
                    && self.grid[ny as usize * w + nx as usize] > 0
                {
                    return true;
                }
            }
        }
        false
    }

    fn crystal_filled(&self) -> f64 {
        let total = self.width * self.height;
        if total == 0 {
            return 1.0;
        }
        let filled = self.grid.iter().filter(|&&v| v > 0).count();
        filled as f64 / total as f64
    }
}

impl Animation for Crystallize {
    fn name(&self) -> &str {
        "crystallize"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.grid = vec![0u8; width * height];
        let cx = width / 2;
        let cy = height / 2;
        if cx < width && cy < height {
            self.grid[cy * width + cx] = 1;
        }
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = self.width;
        let h = self.height;

        self.color_cycle += dt * 0.5;

        // Reset if too full
        if self.crystal_filled() > 0.15 {
            self.growth_timer += dt;
            if self.growth_timer > 5.0 {
                self.grid = vec![0u8; w * h];
                let cx = w / 2;
                let cy = h / 2;
                if cx < w && cy < h {
                    self.grid[cy * w + cx] = 1;
                }
                self.growth_timer = 0.0;
            }
        }

        // Random walk steps
        for _ in 0..self.steps_per_frame {
            for walker in &mut self.walkers {
                walker.0 += self.rng.random_range(-1.5..1.5);
                walker.1 += self.rng.random_range(-1.5..1.5);

                let ix = walker.0 as usize;
                let iy = walker.1 as usize;

                if ix >= w || iy >= h {
                    // Respawn from edge
                    let angle = self.rng.random_range(0.0..std::f64::consts::TAU);
                    let dist = (w.min(h) as f64 * 0.4).max(10.0);
                    walker.0 = w as f64 * 0.5 + angle.cos() * dist;
                    walker.1 = h as f64 * 0.5 + angle.sin() * dist;
                    continue;
                }

                // Inline neighbor check to avoid borrow conflict
                let has_nb = {
                    let mut found = false;
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = ix as i32 + dx;
                            let ny = iy as i32 + dy;
                            if nx >= 0
                                && nx < w as i32
                                && ny >= 0
                                && ny < h as i32
                                && self.grid[ny as usize * w + nx as usize] > 0
                            {
                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                    }
                    found
                };
                if has_nb && self.grid[iy * w + ix] == 0 {
                    // Attach to crystal
                    let color_val = ((self.color_cycle + (ix as f64 + iy as f64) * 0.01).sin()
                        * 127.0
                        + 128.0) as u8;
                    self.grid[iy * w + ix] = color_val.max(1);

                    // Respawn walker
                    let angle = self.rng.random_range(0.0..std::f64::consts::TAU);
                    let dist = (w.min(h) as f64 * 0.4).max(10.0);
                    walker.0 = w as f64 * 0.5 + angle.cos() * dist;
                    walker.1 = h as f64 * 0.5 + angle.sin() * dist;
                }
            }
        }

        // Render
        canvas.clear();
        for y in 0..h.min(canvas.height) {
            for x in 0..w.min(canvas.width) {
                let v = self.grid[y * w + x];
                if v > 0 {
                    let t = v as f64 / 255.0;
                    let hue = (t + time * 0.05).fract();
                    let (r, g, b) = hsv_to_rgb(hue, 0.7, 0.9);
                    canvas.set_colored(x, y, 0.8, r, g, b);
                }
            }
        }

        // Draw walkers as dim dots
        for walker in &self.walkers {
            let px = walker.0 as usize;
            let py = walker.1 as usize;
            if px < canvas.width && py < canvas.height {
                canvas.set_colored(px, py, 0.03, 40, 40, 50);
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
