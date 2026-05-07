use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Gray-Scott reaction-diffusion producing organic coral/brain-like patterns.
///
/// Simulation runs on a downsampled grid (1/4 canvas dimensions) for performance,
/// then is upscaled during rendering.
pub struct ReactionDiffusion {
    #[allow(dead_code)]
    canvas_width: usize,
    #[allow(dead_code)]
    canvas_height: usize,
    sim_width: usize,
    sim_height: usize,
    /// Substrate concentration (starts at 1.0 everywhere)
    u: Vec<f64>,
    /// Catalyst concentration (starts at 0.0, seeded in patches)
    v: Vec<f64>,
    /// Next-frame buffer for U
    next_u: Vec<f64>,
    /// Next-frame buffer for V
    next_v: Vec<f64>,
    /// Feed rate (f)
    feed: f64,
    /// Kill rate (k)
    kill: f64,
    /// Diffusion rate for U
    du: f64,
    /// Diffusion rate for V
    dv: f64,
    /// Timer for simulation steps
    step_timer: f64,
    /// Seconds between simulation steps
    step_interval: f64,
    /// Steps per frame tick
    steps_per_tick: usize,
    /// Timer for auto-reset
    reset_timer: f64,
    /// Seconds before auto-reset
    reset_duration: f64,
    rng: rand::rngs::ThreadRng,
}

impl ReactionDiffusion {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let sim_w = (width / 4).max(20);
        let sim_h = (height / 4).max(12);
        let size = sim_w * sim_h;

        let mut rd = ReactionDiffusion {
            canvas_width: width,
            canvas_height: height,
            sim_width: sim_w,
            sim_height: sim_h,
            u: vec![1.0; size],
            v: vec![0.0; size],
            next_u: vec![0.0; size],
            next_v: vec![0.0; size],
            feed: 0.055,
            kill: 0.062,
            du: 1.0,
            dv: 0.5,
            step_timer: 0.0,
            step_interval: 0.03,
            steps_per_tick: 3,
            reset_timer: 0.0,
            reset_duration: 30.0,
            rng: rand::rng(),
        };
        rd.seed();
        rd
    }

    /// Seed V=1.0 in a few small random patches in the center area.
    fn seed(&mut self) {
        self.u.fill(1.0);
        self.v.fill(0.0);

        let patch_count = self.rng.random_range(3..7);
        let cx = self.sim_width as f64 * 0.5;
        let cy = self.sim_height as f64 * 0.5;
        let spread_x = self.sim_width as f64 * 0.3;
        let spread_y = self.sim_height as f64 * 0.3;

        for _ in 0..patch_count {
            let px = (cx + self.rng.random_range(-spread_x..spread_x)) as i32;
            let py = (cy + self.rng.random_range(-spread_y..spread_y)) as i32;
            let patch_size: i32 = self.rng.random_range(2..4);
            for dy in -patch_size..=patch_size {
                for dx in -patch_size..=patch_size {
                    let x = (px + dx).rem_euclid(self.sim_width as i32) as usize;
                    let y = (py + dy).rem_euclid(self.sim_height as i32) as usize;
                    self.v[y * self.sim_width + x] = 1.0;
                }
            }
        }
    }

    /// Compute the Laplacian using a 9-point stencil.
    /// Center weight: -1, adjacent (up/down/left/right): 0.2, diagonal: 0.05
    fn laplacian(grid: &[f64], x: usize, y: usize, w: usize, h: usize) -> f64 {
        let xm1 = (x as i32 - 1).rem_euclid(w as i32) as usize;
        let xp1 = (x + 1) % w;
        let ym1 = (y as i32 - 1).rem_euclid(h as i32) as usize;
        let yp1 = (y + 1) % h;

        let center = grid[y * w + x];
        let left = grid[y * w + xm1];
        let right = grid[y * w + xp1];
        let up = grid[ym1 * w + x];
        let down = grid[yp1 * w + x];
        let ul = grid[ym1 * w + xm1];
        let ur = grid[ym1 * w + xp1];
        let dl = grid[yp1 * w + xm1];
        let dr = grid[yp1 * w + xp1];

        // 9-point stencil: center=-1, adj=0.2, diag=0.05
        -center + 0.2 * (left + right + up + down) + 0.05 * (ul + ur + dl + dr)
    }

    /// Run one simulation step of the Gray-Scott system.
    fn step(&mut self) {
        let w = self.sim_width;
        let h = self.sim_height;
        let f = self.feed;
        let k = self.kill;
        let du = self.du;
        let dv = self.dv;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let u = self.u[idx];
                let v = self.v[idx];

                let lap_u = Self::laplacian(&self.u, x, y, w, h);
                let lap_v = Self::laplacian(&self.v, x, y, w, h);

                let uvv = u * v * v;

                let new_u = u + du * lap_u - uvv + f * (1.0 - u);
                let new_v = v + dv * lap_v + uvv - (f + k) * v;

                self.next_u[idx] = new_u.clamp(0.0, 1.0);
                self.next_v[idx] = new_v.clamp(0.0, 1.0);
            }
        }

        std::mem::swap(&mut self.u, &mut self.next_u);
        std::mem::swap(&mut self.v, &mut self.next_v);
    }
}

impl Animation for ReactionDiffusion {
    fn name(&self) -> &str {
        "reaction_diffusion"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::HalfBlock
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        *self = ReactionDiffusion::new(width, height, 1.0);
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Advance step timer and run simulation steps
        self.step_timer += dt;
        while self.step_timer >= self.step_interval {
            for _ in 0..self.steps_per_tick {
                self.step();
            }
            self.step_timer -= self.step_interval;
        }

        // Auto-reset timer
        self.reset_timer += dt;
        if self.reset_timer >= self.reset_duration {
            self.reset_timer = 0.0;
            self.seed();
        }

        canvas.clear();

        let cw = canvas.width;
        let ch = canvas.height;
        let sw = self.sim_width;
        let sh = self.sim_height;

        // Scale factors: map canvas pixel to simulation cell
        let sx = sw as f64 / cw as f64;
        let sy = sh as f64 / ch as f64;

        for cy in 0..ch {
            // Map canvas y to sim y
            let sim_y = ((cy as f64 * sy) as usize).min(sh - 1);
            for cx in 0..cw {
                let sim_x = ((cx as f64 * sx) as usize).min(sw - 1);
                let v = self.v[sim_y * sw + sim_x];

                if v < 0.01 {
                    continue;
                }

                // Map V concentration to a color palette:
                // low V -> dark blue, mid -> cyan/green, high -> yellow/white
                let (r, g, b) = if v < 0.15 {
                    // Dark blue to blue
                    let t = v / 0.15;
                    hsv_to_rgb(0.6, 0.8, t * 0.6)
                } else if v < 0.3 {
                    // Blue to cyan
                    let t = (v - 0.15) / 0.15;
                    hsv_to_rgb(0.55 - t * 0.05, 0.8, 0.6 + t * 0.2)
                } else if v < 0.5 {
                    // Cyan to green
                    let t = (v - 0.3) / 0.2;
                    hsv_to_rgb(0.5 - t * 0.15, 0.75, 0.8 + t * 0.1)
                } else if v < 0.7 {
                    // Green to yellow
                    let t = (v - 0.5) / 0.2;
                    hsv_to_rgb(0.35 - t * 0.2, 0.8 - t * 0.1, 0.9 + t * 0.05)
                } else {
                    // Yellow to white
                    let t = ((v - 0.7) / 0.3).min(1.0);
                    hsv_to_rgb(0.15 - t * 0.15, 0.7 - t * 0.7, 1.0)
                };

                let brightness = (v * 1.2).clamp(0.0, 1.0);
                canvas.set_colored(cx, cy, brightness, r, g, b);
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((h % 1.0) + 1.0) % 1.0;
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
