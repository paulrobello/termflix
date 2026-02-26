use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct RippleSource {
    x: f64,
    y: f64,
    birth: f64,
    strength: f64,
}

/// Ripple interference pattern (like water drops)
pub struct Ripple {
    sources: Vec<RippleSource>,
    spawn_timer: f64,
    rng: rand::rngs::ThreadRng,
}

impl Ripple {
    pub fn new(_width: usize, _height: usize) -> Self {
        Ripple {
            sources: Vec::new(),
            spawn_timer: 0.0,
            rng: rand::rng(),
        }
    }
}

impl Animation for Ripple {
    fn name(&self) -> &str {
        "ripple"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;

        // Spawn new ripple sources
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            self.sources.push(RippleSource {
                x: self.rng.random_range(0.0..w),
                y: self.rng.random_range(0.0..h),
                birth: time,
                strength: self.rng.random_range(0.5..1.0),
            });
            self.spawn_timer = self.rng.random_range(0.3..1.5);
        }

        // Remove old sources
        self.sources.retain(|s| time - s.birth < 8.0);

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                let mut val = 0.0;
                for src in &self.sources {
                    let age = time - src.birth;
                    let dx = fx - src.x;
                    let dy = fy - src.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    let wave_front = age * 30.0;
                    let ring_dist = (dist - wave_front).abs();

                    if ring_dist < 15.0 {
                        let decay = (-age * 0.4).exp(); // fade over time
                        let spatial_decay = (-ring_dist * 0.2).exp();
                        let wave = (dist * 0.5 - age * 15.0).sin();
                        val += wave * decay * spatial_decay * src.strength;
                    }
                }

                let v = ((val + 1.0) * 0.5).clamp(0.0, 1.0);
                if v > 0.05 {
                    // Blue-cyan color scheme
                    let r = (40.0 * v) as u8;
                    let g = (120.0 + 135.0 * v) as u8;
                    let b = (180.0 + 75.0 * v) as u8;
                    canvas.set_colored(x, y, v, r, g, b);
                }
            }
        }
    }
}
