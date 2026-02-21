use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

struct Firefly {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    phase: f64,       // blink phase
    blink_speed: f64,  // how fast it blinks
    glow_radius: f64,
}

/// Fireflies blinking in the dark
pub struct Fireflies {
    width: usize,
    height: usize,
    flies: Vec<Firefly>,
}

impl Fireflies {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let count = ((width * height) as f64 / 200.0 * scale) as usize;
        let flies = (0..count.max(10))
            .map(|_| Firefly {
                x: rng.random_range(0.0..width as f64),
                y: rng.random_range(0.0..height as f64),
                vx: rng.random_range(-3.0..3.0),
                vy: rng.random_range(-3.0..3.0),
                phase: rng.random_range(0.0..std::f64::consts::TAU),
                blink_speed: rng.random_range(1.0..4.0),
                glow_radius: rng.random_range(2.0..5.0),
            })
            .collect();

        Fireflies {
            width,
            height,
            flies,
        }
    }
}

impl Animation for Fireflies {
    fn name(&self) -> &str { "fireflies" }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;

        canvas.clear();

        // Slight ambient glow for atmosphere
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                canvas.set_colored(x, y, 0.02, 5, 10, 15);
            }
        }

        for fly in &mut self.flies {
            // Wandering movement
            fly.vx += rng.random_range(-2.0..2.0) * dt;
            fly.vy += rng.random_range(-2.0..2.0) * dt;
            fly.vx *= 0.98;
            fly.vy *= 0.98;
            fly.x += fly.vx * dt * 5.0;
            fly.y += fly.vy * dt * 5.0;

            // Wrap around
            if fly.x < 0.0 { fly.x += self.width as f64; }
            if fly.x >= self.width as f64 { fly.x -= self.width as f64; }
            if fly.y < 0.0 { fly.y += self.height as f64; }
            if fly.y >= self.height as f64 { fly.y -= self.height as f64; }

            // Blink pattern: sharp on/off with smooth glow
            let blink = ((time * fly.blink_speed + fly.phase).sin() + 0.3).clamp(0.0, 1.0);
            let on = blink > 0.6;

            if !on {
                continue;
            }

            let intensity = ((blink - 0.6) / 0.4).clamp(0.0, 1.0);
            let radius = fly.glow_radius * intensity;

            // Draw glow
            let r = radius.ceil() as i32;
            for dy in -r..=r {
                for dx in -r..=r {
                    let px = (fly.x as i32 + dx) as usize;
                    let py = (fly.y as i32 + dy) as usize;
                    if px < canvas.width && py < canvas.height {
                        let dist = ((dx * dx + dy * dy) as f64).sqrt();
                        if dist <= radius {
                            let falloff = 1.0 - (dist / radius);
                            let glow = falloff * falloff * intensity;
                            if glow > 0.05 {
                                // Warm yellow-green
                                let gr = (180.0 * glow) as u8;
                                let gg = (255.0 * glow) as u8;
                                let gb = (50.0 * glow) as u8;
                                let idx = py * canvas.width + px;
                                let existing = canvas.pixels[idx];
                                if glow > existing {
                                    canvas.set_colored(px, py, glow, gr, gg, gb);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
