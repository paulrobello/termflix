use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Classic Doom-style fire effect
pub struct Fire {
    width: usize,
    height: usize,
    buffer: Vec<f64>,
    /// Heat rate: controls how hot the bottom row burns (0.0 = cold, 1.0 = normal, 2.0 = intense)
    heat_rate: f64,
    rng: rand::rngs::ThreadRng,
}

impl Fire {
    pub fn new(width: usize, height: usize) -> Self {
        let mut buffer = vec![0.0; width * height];
        // Seed bottom rows
        for x in 0..width {
            for y in height.saturating_sub(2)..height {
                buffer[y * width + x] = 1.0;
            }
        }
        Fire {
            width,
            height,
            buffer,
            heat_rate: 0.8,
            rng: rand::rng(),
        }
    }
}

impl Animation for Fire {
    fn name(&self) -> &str {
        "fire"
    }

    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(intensity) = params.intensity {
            self.heat_rate = intensity.clamp(0.0, 2.0);
        }
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.buffer = vec![0.0; width * height];
        for x in 0..width {
            for y in height.saturating_sub(2)..height {
                self.buffer[y * width + x] = 1.0;
            }
        }
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, _time: f64) {
        let w = self.width;
        let h = self.height;

        // heat_rate > 1.0: reduce decay so fire reaches higher (more intense)
        // heat_rate < 1.0: normal decay but cooler bottom source
        let intensity_scale = 1.0 / self.heat_rate.clamp(0.1, 2.0);

        // Classic Doom fire: for each pixel, pull heat from below
        // Process bottom-to-top so heat propagates fully in one frame
        for x in 0..w {
            for y in 0..h.saturating_sub(1) {
                let wind: i32 = self.rng.random_range(-1i32..=1);
                let src_x = (x as i32 + wind).clamp(0, w as i32 - 1) as usize;
                let src_y = y + 1;
                let src_val = self.buffer[src_y * w + src_x];
                // Scale decay to canvas height so fire reaches ~60% up at heat_rate=1.0
                // At heat_rate=2.0, intensity_scale=0.5 → half decay → fire reaches higher
                let max_decay = (3.0 / h as f64) * intensity_scale;
                let decay = self.rng.random_range(0.0..max_decay.max(f64::EPSILON));
                self.buffer[y * w + x] = (src_val - decay).max(0.0);
            }
        }

        // Keep bottom row hot — capped at 1.0 for brightness, but decay scaling above
        // gives heat_rate > 1.0 its extra reach
        let heat = self.heat_rate.min(1.0);
        let heat_min = (heat * 0.9).max(0.0);
        for x in 0..w {
            self.buffer[(h - 1) * w + x] = if heat_min < heat {
                self.rng.random_range(heat_min..heat)
            } else {
                heat
            };
        }

        // Draw to canvas
        canvas.clear();
        for y in 0..h {
            for x in 0..w {
                let v = self.buffer[y * w + x];
                if v > 0.01 {
                    let (r, g, b) = fire_color(v);
                    canvas.set_colored(x, y, v, r, g, b);
                }
            }
        }
    }
}

fn fire_color(v: f64) -> (u8, u8, u8) {
    if v > 0.85 {
        let t = (v - 0.85) / 0.15;
        (255, (200.0 + 55.0 * t) as u8, (t * 200.0) as u8)
    } else if v > 0.6 {
        let t = (v - 0.6) / 0.25;
        (255, (t * 200.0) as u8, 0)
    } else if v > 0.3 {
        let t = (v - 0.3) / 0.3;
        ((100.0 + 155.0 * t) as u8, 0, 0)
    } else {
        let t = v / 0.3;
        ((t * 100.0) as u8, 0, 0)
    }
}
