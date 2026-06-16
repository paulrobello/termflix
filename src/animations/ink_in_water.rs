use super::Animation;
use crate::render::Canvas;
use noise::{NoiseFn, Perlin};
use rand::RngExt;

const STEP_INTERVAL: f64 = 1.0 / 30.0;
const DECAY: f64 = 0.94;
const DMAX: f64 = 4.0;
const PUFF_INTERVAL: f64 = 2.0;
const PUFF_SIZE: usize = 90;
const MAX_PARTICLES: usize = 500;
const SPEED: f64 = 14.0;
const MAX_STEPS_PER_FRAME: usize = 4;

struct InkParticle {
    x: f64,
    y: f64,
    life: f64,
    max_life: f64,
    r: u8,
    g: u8,
    b: u8,
}

/// Ink in water: colored ink puffs advected through a Perlin flow field,
/// depositing into a slowly diffusing density grid.
pub struct InkInWater {
    width: usize,
    height: usize,
    density: Vec<f64>,
    color: Vec<(f64, f64, f64)>,
    particles: Vec<InkParticle>,
    noise: Perlin,
    puff_timer: f64,
    step_timer: f64,
}

impl InkInWater {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = scale;
        let n = width * height;
        let mut ink = InkInWater {
            width,
            height,
            density: vec![0.0; n],
            color: vec![(0.0, 0.0, 0.0); n],
            particles: Vec::new(),
            noise: Perlin::new(rand::rng().random_range(0..u32::MAX)),
            puff_timer: 0.0,
            step_timer: 0.0,
        };
        for _ in 0..3 {
            ink.spawn_puff();
        }
        ink
    }

    /// Spawn a single colored ink puff at a random location.
    fn spawn_puff(&mut self) {
        let mut rng = rand::rng();
        let cx = rng.random_range(self.width as f64 * 0.25..self.width as f64 * 0.75);
        let cy = rng.random_range(self.height as f64 * 0.25..self.height as f64 * 0.75);
        let hue = rng.random_range(0.0..1.0);
        let (r, g, b) = hsv_to_rgb(hue, 0.9, 1.0);
        for _ in 0..PUFF_SIZE {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }
            let a = rng.random_range(0.0..std::f64::consts::TAU);
            let rad = rng.random_range(0.0..2.5);
            let life = rng.random_range(2.5..5.0);
            self.particles.push(InkParticle {
                x: cx + a.cos() * rad,
                y: cy + a.sin() * rad,
                life,
                max_life: life,
                r,
                g,
                b,
            });
        }
    }

    fn step(&mut self, dt: f64, time: f64) {
        let t = time * 0.25;
        let ns = 0.018;
        let w = self.width;
        let h = self.height;
        let tau = std::f64::consts::TAU;

        for p in &mut self.particles {
            let angle = self.noise.get([p.x * ns, p.y * ns, t]) * tau * 1.5;
            p.x += angle.cos() * SPEED * dt;
            p.y += angle.sin() * SPEED * dt;
            p.life -= dt;

            if p.x >= 0.0 && p.y >= 0.0 {
                let ix = p.x as usize;
                let iy = p.y as usize;
                if ix < w && iy < h {
                    let wgt = (p.life / p.max_life).clamp(0.0, 1.0) * 0.6;
                    let idx = iy * w + ix;
                    self.density[idx] += wgt;
                    self.color[idx].0 += wgt * p.r as f64;
                    self.color[idx].1 += wgt * p.g as f64;
                    self.color[idx].2 += wgt * p.b as f64;
                }
            }
        }

        self.particles.retain(|p| {
            p.life > 0.0 && p.x >= 0.0 && p.x < w as f64 && p.y >= 0.0 && p.y < h as f64
        });

        for d in &mut self.density {
            *d *= DECAY;
        }
        for c in &mut self.color {
            *c = (c.0 * DECAY, c.1 * DECAY, c.2 * DECAY);
        }
    }
}

impl Animation for InkInWater {
    fn name(&self) -> &str {
        "ink_in_water"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        if canvas.width != self.width || canvas.height != self.height {
            self.width = canvas.width;
            self.height = canvas.height;
            let n = self.width * self.height;
            self.density = vec![0.0; n];
            self.color = vec![(0.0, 0.0, 0.0); n];
            self.particles.clear();
        }

        self.puff_timer += dt;
        if self.puff_timer >= PUFF_INTERVAL {
            self.puff_timer = 0.0;
            if self.particles.len() < MAX_PARTICLES - PUFF_SIZE {
                self.spawn_puff();
            }
        }

        self.step_timer += dt;
        let mut steps = 0;
        while self.step_timer >= STEP_INTERVAL && steps < MAX_STEPS_PER_FRAME {
            self.step(STEP_INTERVAL, time);
            self.step_timer -= STEP_INTERVAL;
            steps += 1;
        }
        if self.step_timer > STEP_INTERVAL * MAX_STEPS_PER_FRAME as f64 {
            self.step_timer = 0.0;
        }

        canvas.clear();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let d = self.density[idx];
                if d > 0.02 {
                    let inten = (d / DMAX).min(1.0);
                    let (cr, cg, cb) = self.color[idx];
                    let r = (cr / d).clamp(0.0, 255.0) as u8;
                    let g = (cg / d).clamp(0.0, 255.0) as u8;
                    let b = (cb / d).clamp(0.0, 255.0) as u8;
                    canvas.set_colored(x, y, inten, r, g, b);
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
