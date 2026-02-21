use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

/// Classic Doom-style fire effect
pub struct Fire {
    width: usize,
    height: usize,
    buffer: Vec<f64>,
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
        Fire { width, height, buffer }
    }

    fn resize(&mut self, w: usize, h: usize) {
        self.width = w;
        self.height = h;
        self.buffer = vec![0.0; w * h];
        for x in 0..w {
            for y in h.saturating_sub(2)..h {
                self.buffer[y * w + x] = 1.0;
            }
        }
    }
}

impl Animation for Fire {
    fn name(&self) -> &str { "fire" }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, _time: f64) {
        let mut rng = rand::rng();
        let w = canvas.width;
        let h = canvas.height;

        if self.width != w || self.height != h {
            self.resize(w, h);
        }

        // Classic Doom fire: for each pixel, pull heat from below
        // Process bottom-to-top so heat propagates fully in one frame
        for x in 0..w {
            for y in 0..h.saturating_sub(1) {
                let wind: i32 = rng.random_range(-1i32..=1);
                let src_x = (x as i32 + wind).clamp(0, w as i32 - 1) as usize;
                let src_y = y + 1;
                let src_val = self.buffer[src_y * w + src_x];
                // Scale decay to canvas height so fire reaches ~60% up
                let max_decay = 3.0 / h as f64;
                let decay = rng.random_range(0.0..max_decay);
                self.buffer[y * w + x] = (src_val - decay).max(0.0);
            }
        }

        // Keep bottom row hot
        for x in 0..w {
            self.buffer[(h - 1) * w + x] = rng.random_range(0.9..1.0);
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
