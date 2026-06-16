use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

const MU_BASE: f64 = 0.15;
const MU_AMP: f64 = 0.06;
const MU_FREQ: f64 = 0.35;
const SIGMA_L: f64 = 0.05;
const DT: f64 = 0.08;
const STEP_INTERVAL: f64 = 1.0 / 20.0;
const DOWNSAMPLE: usize = 2;
const KERNEL_R: i32 = 6;
const PEAK_R: f64 = 3.5;
const WIDTH_K: f64 = 1.5;
/// Seconds between gentle perturbations that keep the field perpetually evolving.
const PERTURB_INTERVAL: f64 = 6.0;

/// Lenia: a continuous generalization of Conway's Game of Life. A soft
/// gaussian-ring kernel convolves a [0,1] grid; a gaussian growth function
/// drives cells toward life. The growth target slowly oscillates and a gentle
/// periodic perturbation is injected, so the field never settles into a static
/// pattern. A low-mass reseed prevents collapse to black.
pub struct Lenia {
    sw: usize,
    sh: usize,
    grid: Vec<f64>,
    back: Vec<f64>,
    kernel: Vec<(i32, i32, f64)>,
    ksum: f64,
    step_timer: f64,
    perturb_timer: f64,
}

impl Lenia {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = scale;
        let mut l = Lenia {
            sw: 1,
            sh: 1,
            grid: Vec::new(),
            back: Vec::new(),
            kernel: Vec::new(),
            ksum: 1.0,
            step_timer: 0.0,
            perturb_timer: 0.0,
        };
        l.build_kernel();
        l.init_grid(width.max(1), height.max(1));
        l
    }

    fn build_kernel(&mut self) {
        self.kernel.clear();
        let mut sum = 0.0;
        let two_w2 = 2.0 * WIDTH_K * WIDTH_K;
        for dy in -KERNEL_R..=KERNEL_R {
            for dx in -KERNEL_R..=KERNEL_R {
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist > KERNEL_R as f64 {
                    continue;
                }
                let w = (-((dist - PEAK_R).powi(2)) / two_w2).exp();
                if w > 0.01 {
                    self.kernel.push((dx, dy, w));
                    sum += w;
                }
            }
        }
        self.ksum = sum.max(1e-6);
    }

    fn init_grid(&mut self, width: usize, height: usize) {
        self.sw = (width / DOWNSAMPLE).max(8);
        self.sh = (height / DOWNSAMPLE).max(8);
        let n = self.sw * self.sh;
        self.grid = vec![0.0; n];
        self.back = vec![0.0; n];
        self.seed_random(8);
        // Faint global texture so empty regions have something to evolve.
        let mut rng = rand::rng();
        for v in &mut self.grid {
            *v = (*v + rng.random_range(0.0..0.12)).clamp(0.0, 1.0);
        }
    }

    /// Stamp gaussian disks of life at random positions.
    fn seed_random(&mut self, count: usize) {
        let mut rng = rand::rng();
        for _ in 0..count {
            let bx = rng.random_range(0.0..self.sw as f64);
            let by = rng.random_range(0.0..self.sh as f64);
            let rad = rng.random_range(4.0..7.0);
            let amp = rng.random_range(0.45..0.75);
            let two_r2 = 2.0 * rad * rad;
            for y in 0..self.sh {
                for x in 0..self.sw {
                    let d = ((x as f64 - bx).powi(2) + (y as f64 - by).powi(2)).sqrt();
                    let v = amp * (-(d * d) / two_r2).exp();
                    let idx = y * self.sw + x;
                    self.grid[idx] = (self.grid[idx] + v).clamp(0.0, 1.0);
                }
            }
        }
    }

    /// One CA tick with the time-varying growth target `mu`.
    fn step(&mut self, time: f64) {
        let mu = MU_BASE + MU_AMP * (time * MU_FREQ).sin();
        let sw = self.sw;
        let sh = self.sh;
        let swi = sw as i32;
        let shi = sh as i32;
        for y in 0..sh {
            for x in 0..sw {
                let mut u = 0.0;
                for &(dx, dy, w) in &self.kernel {
                    let nx = ((x as i32 + dx).rem_euclid(swi)) as usize;
                    let ny = ((y as i32 + dy).rem_euclid(shi)) as usize;
                    u += self.grid[ny * sw + nx] * w;
                }
                u /= self.ksum;
                let g = 2.0 * (-((u - mu) / SIGMA_L).powi(2)).exp() - 1.0;
                self.back[y * sw + x] = (self.grid[y * sw + x] + DT * g).clamp(0.0, 1.0);
            }
        }
        std::mem::swap(&mut self.grid, &mut self.back);

        // Emergency reseed so the field never collapses to black.
        let mut mass = 0usize;
        for &v in &self.grid {
            if v > 0.15 {
                mass += 1;
            }
        }
        if mass < 20 {
            self.seed_random(4);
        }
    }
}

impl Animation for Lenia {
    fn name(&self) -> &str {
        "lenia"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let target_sw = (canvas.width / DOWNSAMPLE).max(8);
        let target_sh = (canvas.height / DOWNSAMPLE).max(8);
        if target_sw != self.sw || target_sh != self.sh {
            self.init_grid(canvas.width.max(1), canvas.height.max(1));
        }

        // Gentle periodic perturbation keeps the system perpetually evolving.
        self.perturb_timer += dt;
        if self.perturb_timer >= PERTURB_INTERVAL {
            self.perturb_timer = 0.0;
            self.seed_random(2);
        }

        self.step_timer += dt;
        let mut steps = 0;
        while self.step_timer >= STEP_INTERVAL && steps < 3 {
            self.step(time);
            self.step_timer -= STEP_INTERVAL;
            steps += 1;
        }
        if self.step_timer > STEP_INTERVAL * 3.0 {
            self.step_timer = 0.0;
        }

        canvas.clear();
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let sx = (x / DOWNSAMPLE).min(self.sw - 1);
                let sy = (y / DOWNSAMPLE).min(self.sh - 1);
                let v = self.grid[sy * self.sw + sx];
                if v > 0.03 {
                    let hue = (1.0 - v) * 0.8;
                    let (r, g, b) = hsv_to_rgb(hue, 0.85, 0.4 + 0.6 * v);
                    canvas.set_colored(x, y, v, r, g, b);
                }
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = h.rem_euclid(1.0);
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
