use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

struct Blob {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    radius: f64,
}

/// Lava lamp with metaball blobs rising, merging, and splitting
pub struct Lava {
    width: usize,
    height: usize,
    blobs: Vec<Blob>,
}

impl Lava {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let count = (8.0 * scale) as usize;
        let w = width as f64;
        let h = height as f64;
        let blobs = (0..count)
            .map(|_| Blob {
                x: rng.random_range(w * 0.2..w * 0.8),
                y: rng.random_range(0.0..h),
                vx: rng.random_range(-3.0..3.0),
                vy: rng.random_range(-8.0..-2.0),
                radius: rng.random_range(4.0..10.0),
            })
            .collect();
        Lava { width, height, blobs }
    }
}

impl Animation for Lava {
    fn name(&self) -> &str {
        "lava"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;
        let w = self.width as f64;
        let h = self.height as f64;

        // Update blob positions
        for blob in &mut self.blobs {
            blob.x += blob.vx * dt;
            blob.y += blob.vy * dt;

            // Gentle horizontal wobble
            blob.vx += rng.random_range(-1.0..1.0) * dt * 5.0;
            blob.vx = blob.vx.clamp(-5.0, 5.0);

            // Buoyancy: rise when low, sink when high
            let center_y = h * 0.5;
            blob.vy += (center_y - blob.y) * 0.01 * dt;
            blob.vy += rng.random_range(-2.0..2.0) * dt;
            blob.vy = blob.vy.clamp(-10.0, 10.0);

            // Radius pulsing
            blob.radius = (blob.radius + rng.random_range(-0.5..0.5) * dt).clamp(3.0, 12.0);

            // Bounce off walls
            if blob.x < blob.radius {
                blob.x = blob.radius;
                blob.vx = blob.vx.abs();
            }
            if blob.x > w - blob.radius {
                blob.x = w - blob.radius;
                blob.vx = -blob.vx.abs();
            }
            if blob.y < blob.radius {
                blob.y = blob.radius;
                blob.vy = blob.vy.abs();
            }
            if blob.y > h - blob.radius {
                blob.y = h - blob.radius;
                blob.vy = -blob.vy.abs();
            }
        }

        canvas.clear();

        // Render metaball field
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                // Sum metaball contributions
                let mut field = 0.0;
                for blob in &self.blobs {
                    let dx = fx - blob.x;
                    let dy = fy - blob.y;
                    let dist_sq = dx * dx + dy * dy;
                    field += (blob.radius * blob.radius) / (dist_sq + 1.0);
                }

                // Add a slow-moving background wave
                let bg = ((fx * 0.05 + time * 0.3).sin() * 0.1 + 0.1).max(0.0);
                field += bg;

                if field > 0.3 {
                    let v = ((field - 0.3) / 1.5).clamp(0.0, 1.0);
                    let (r, g, b) = lava_color(v, time, fx, fy);
                    canvas.set_colored(x, y, v, r, g, b);
                }
            }
        }
    }
}

fn lava_color(v: f64, time: f64, x: f64, y: f64) -> (u8, u8, u8) {
    let shift = ((x * 0.02 + y * 0.01 + time * 0.2).sin() * 0.5 + 0.5) * 0.2;
    let t = (v + shift).clamp(0.0, 1.0);
    if t > 0.92 {
        // Tiny hot orange-yellow core (no white!)
        let f = (t - 0.92) / 0.08;
        (255, (120.0 + 80.0 * f) as u8, (20.0 + 40.0 * f) as u8)
    } else if t > 0.7 {
        // Bright red-orange
        let f = (t - 0.7) / 0.22;
        ((180.0 + 75.0 * f) as u8, (30.0 + 90.0 * f) as u8, 0)
    } else if t > 0.4 {
        // Deep red
        let f = (t - 0.4) / 0.3;
        ((60.0 + 120.0 * f) as u8, (5.0 + 25.0 * f) as u8, 0)
    } else if t > 0.15 {
        // Very dark red / maroon
        let f = (t - 0.15) / 0.25;
        ((20.0 + 40.0 * f) as u8, 0, 0)
    } else {
        // Near black with faint red
        let f = t / 0.15;
        ((f * 20.0) as u8, 0, 0)
    }
}
