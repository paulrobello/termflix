use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

struct SandParticle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    size: f64,
}

/// Blowing sand particles with dune formation at bottom
pub struct Sandstorm {
    width: usize,
    height: usize,
    particles: Vec<SandParticle>,
    dunes: Vec<f64>,
    wind: f64,
    wind_target: f64,
    wind_timer: f64,
}

impl Sandstorm {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let count = ((width * height) as f64 / 60.0 * scale) as usize;
        let particles = (0..count)
            .map(|_| SandParticle {
                x: rng.random_range(0.0..width as f64),
                y: rng.random_range(0.0..height as f64),
                vx: rng.random_range(5.0..15.0),
                vy: rng.random_range(-1.0..2.0),
                size: rng.random_range(0.5..1.0),
            })
            .collect();
        let dunes = vec![0.0; width];
        Sandstorm {
            width,
            height,
            particles,
            dunes,
            wind: 10.0,
            wind_target: 10.0,
            wind_timer: 0.0,
        }
    }
}

impl Animation for Sandstorm {
    fn name(&self) -> &str {
        "sandstorm"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;
        let w = self.width as f64;
        let h = self.height as f64;

        // Resize dunes if needed
        self.dunes.resize(self.width, 0.0);

        // Vary wind
        self.wind_timer -= dt;
        if self.wind_timer <= 0.0 {
            self.wind_target = rng.random_range(5.0..25.0);
            self.wind_timer = rng.random_range(1.0..4.0);
        }
        self.wind += (self.wind_target - self.wind) * dt * 0.8;

        canvas.clear();

        // Update particles
        for p in &mut self.particles {
            let gust = (p.y * 0.1 + p.x * 0.05).sin() * 3.0;
            p.vx = self.wind + gust + rng.random_range(-2.0..2.0);
            p.vy += rng.random_range(-0.5..1.5) * dt * 10.0;
            p.vy = p.vy.clamp(-2.0, 8.0);

            p.x += p.vx * dt;
            p.y += p.vy * dt;

            // Check dune collision
            let ix = (p.x as usize).min(self.width.saturating_sub(1));
            let dune_top = h - self.dunes[ix];
            if p.y >= dune_top && p.vy > 0.0 {
                // Deposit sand — spread across neighbors for natural look
                let amt = 0.08;
                self.dunes[ix] = (self.dunes[ix] + amt).min(h * 0.4);
                if ix > 0 { self.dunes[ix - 1] = (self.dunes[ix - 1] + amt * 0.5).min(h * 0.4); }
                if ix + 1 < self.width { self.dunes[ix + 1] = (self.dunes[ix + 1] + amt * 0.5).min(h * 0.4); }
                // Reset particle
                p.x = rng.random_range(-10.0..0.0);
                p.y = rng.random_range(0.0..h * 0.8);
                p.vy = rng.random_range(-1.0..2.0);
                continue;
            }

            // Wrap horizontally
            if p.x >= w {
                p.x -= w + 10.0;
                p.y = rng.random_range(0.0..h * 0.8);
            }
            if p.x < -10.0 {
                p.x += w + 10.0;
            }
            if p.y >= h {
                p.y = 0.0;
                p.x = rng.random_range(0.0..w);
            }
            if p.y < 0.0 {
                p.y = 0.0;
                p.vy = p.vy.abs();
            }

            // Draw particle
            let px = p.x as usize;
            let py = p.y as usize;
            if px < canvas.width && py < canvas.height {
                let brightness = p.size * 0.8;
                let shade = rng.random_range(0.8..1.0);
                let r = (210.0 * shade) as u8;
                let g = (180.0 * shade) as u8;
                let b = (120.0 * shade) as u8;
                canvas.set_colored(px, py, brightness, r, g, b);
            }
        }

        // Smooth dunes — diffusion pass to prevent spiky columns
        let mut new_dunes = self.dunes.clone();
        let len = self.dunes.len();
        for i in 0..len {
            let left = if i > 0 { self.dunes[i - 1] } else { self.dunes[i] };
            let right = if i + 1 < len { self.dunes[i + 1] } else { self.dunes[i] };
            // Blend with neighbors
            new_dunes[i] = self.dunes[i] * 0.5 + (left + right) * 0.25;
        }
        self.dunes = new_dunes;

        // Avalanche — sand slides if slope is too steep
        for _ in 0..3 {
            for i in 0..len.saturating_sub(1) {
                let diff = self.dunes[i] - self.dunes[i + 1];
                if diff.abs() > 1.5 {
                    let transfer = diff * 0.3;
                    self.dunes[i] -= transfer;
                    self.dunes[i + 1] += transfer;
                }
            }
        }

        // Wind erosion
        for i in 0..len {
            if self.dunes[i] > 0.0 {
                let erosion = self.wind * 0.002 * dt;
                self.dunes[i] = (self.dunes[i] - erosion).max(0.0);
                let ni = (i + 1) % len;
                self.dunes[ni] += erosion * 0.5;
            }
        }

        // Draw dunes
        for x in 0..self.width.min(canvas.width) {
            let dune_h = self.dunes[x];
            let top = (h - dune_h) as usize;
            for y in top..canvas.height {
                let depth = (y - top) as f64 / dune_h.max(1.0);
                let shade = 1.0 - depth * 0.3;
                let r = (194.0 * shade) as u8;
                let g = (160.0 * shade) as u8;
                let b = (100.0 * shade) as u8;
                canvas.set_colored(x, y, 0.8, r, g, b);
            }
        }
    }
}
