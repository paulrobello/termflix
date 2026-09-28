use super::Animation;
use crate::color::hsv_to_rgb;
use crate::render::Canvas;
use rand::RngExt;

/// Downward acceleration applied to balls (pixels / second^2).
const GRAVITY: f64 = 130.0;
/// Terminal fall speed (pixels / second).
const MAX_VY: f64 = 90.0;
/// Seconds between ball drops.
const SPAWN_INTERVAL: f64 = 0.22;
/// Cap on simultaneously falling balls.
const MAX_BALLS: usize = 140;
/// Drop count before the histogram resets for a fresh run.
const RESET_EVERY: usize = 500;

struct Ball {
    row: i32,
    slot: usize,
    x: f64,
    y: f64,
    vy: f64,
    hue: f64,
}

/// Galton board: balls cascade through a peg grid and accumulate into bins,
/// forming an emergent bell curve (normal distribution).
pub struct Galton {
    balls: Vec<Ball>,
    bins: Vec<usize>,
    n_rows: i32,
    spawn_timer: f64,
    total_collected: usize,
    rng: rand::rngs::StdRng,
}

impl Galton {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = (width, height, scale);
        Galton {
            balls: Vec::new(),
            bins: Vec::new(),
            n_rows: 0,
            spawn_timer: 0.0,
            total_collected: 0,
            rng: crate::rng::new_rng(),
        }
    }

    /// Derive peg-field geometry from the current canvas size.
    /// Returns (y0, y_bins, row_spacing, col_spacing, n_rows).
    fn layout(canvas: &Canvas) -> (f64, f64, f64, f64, i32) {
        let h = canvas.height as f64;
        let y0 = h * 0.16;
        let y_bins = h * 0.80;
        let row_spacing = 2.4;
        let col_spacing = 3.2;
        let field = (y_bins - y0).max(1.0);
        let n_rows = ((field / row_spacing) as i32).clamp(6, 12);
        (y0, y_bins, row_spacing, col_spacing, n_rows)
    }

    /// Horizontal pixel position of a peg at (row, slot).
    fn peg_x(center: f64, col_spacing: f64, row: i32, slot: usize) -> f64 {
        center + (slot as f64 - row as f64 * 0.5) * col_spacing
    }
}

impl Animation for Galton {
    fn name(&self) -> &str {
        "galton"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let center = w * 0.5;
        let (y0, y_bins, row_spacing, col_spacing, n_rows) = Self::layout(canvas);

        // Resize bins if the peg count changed (e.g. terminal resized).
        let need = (n_rows + 1) as usize;
        if self.bins.len() != need {
            self.bins = vec![0usize; need];
            self.balls.clear();
        }
        self.n_rows = n_rows;

        canvas.clear();
        let rng = &mut self.rng;

        // Spawn new balls from the hopper.
        self.spawn_timer += dt;
        while self.spawn_timer >= SPAWN_INTERVAL && self.balls.len() < MAX_BALLS {
            self.spawn_timer -= SPAWN_INTERVAL;
            self.balls.push(Ball {
                row: -1,
                slot: 0,
                x: center,
                y: y0 - row_spacing,
                vy: 0.0,
                hue: rng.random_range(0.0..1.0),
            });
        }
        if self.spawn_timer > SPAWN_INTERVAL {
            self.spawn_timer = SPAWN_INTERVAL;
        }

        // Advance balls; collect bin indices for those that land.
        let mut collected: Vec<usize> = Vec::new();
        for b in &mut self.balls {
            b.vy = (b.vy + GRAVITY * dt).min(MAX_VY);
            b.y += b.vy * dt;
            loop {
                let next_row_y = y0 + (b.row as f64 + 1.0) * row_spacing;
                if b.y < next_row_y {
                    break;
                }
                b.row += 1;
                b.y = next_row_y;
                if b.row >= 1 {
                    // Decide left or right at this peg.
                    if rng.random_range(0.0..1.0) >= 0.5 {
                        b.slot += 1;
                    }
                    b.x = Self::peg_x(center, col_spacing, b.row, b.slot);
                }
                if b.row >= n_rows {
                    collected.push(b.slot.min(need - 1));
                    break;
                }
            }
        }

        // Tally collections, drop landed balls, refresh periodically.
        for c in collected {
            self.bins[c] += 1;
            self.total_collected += 1;
        }
        self.balls.retain(|b| b.row < n_rows);
        if self.total_collected >= RESET_EVERY {
            self.bins.fill(0);
            self.total_collected = 0;
        }

        // Draw pegs.
        for row in 0..n_rows {
            let py = y0 + row as f64 * row_spacing;
            for slot in 0..=row as usize {
                let px = Self::peg_x(center, col_spacing, row, slot);
                let xi = px as usize;
                let yi = py as usize;
                if xi < canvas.width && yi < canvas.height {
                    canvas.set_colored(xi, yi, 0.25, 90, 95, 115);
                }
            }
        }

        // Draw falling balls.
        for b in &self.balls {
            let (r, g, bl) = hsv_to_rgb(b.hue, 0.85, 1.0);
            let xi = b.x as usize;
            let yi = b.y as usize;
            if xi < canvas.width && yi < canvas.height {
                canvas.set_colored(xi, yi, 0.95, r, g, bl);
            }
        }

        // Draw histogram bins (bars grow from the bottom).
        let max_count = (*self.bins.iter().max().unwrap_or(&0)).max(1) as f64;
        let bin_zone_h = (h - y_bins).max(1.0);
        for (i, &count) in self.bins.iter().enumerate() {
            let bx = Self::peg_x(center, col_spacing, n_rows, i);
            let frac = count as f64 / max_count;
            let bar_h = frac * bin_zone_h;
            let top = (h - bar_h) as i64;
            let hue = i as f64 / need as f64;
            let (r, g, bl) = hsv_to_rgb(hue, 0.8, 0.95);
            let xxi = bx as i64;
            for yy in top..=(h as i64) {
                for dxx in -1i64..=1 {
                    let xx = (xxi + dxx) as usize;
                    let yyu = yy as usize;
                    if xx < canvas.width && yyu < canvas.height {
                        canvas.set_colored(xx, yyu, 0.6, r, g, bl);
                    }
                }
            }
        }
    }
}
