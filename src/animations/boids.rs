use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Boid {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    hue: f64,
}

/// Boids flocking simulation
pub struct Boids {
    width: usize,
    height: usize,
    boids: Vec<Boid>,
}

impl Boids {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let count = (((width * height) as f64 / 150.0 * scale) as usize).clamp(20, 300);
        let boids = (0..count)
            .map(|_| {
                let angle = rng.random_range(0.0..std::f64::consts::TAU);
                let speed = rng.random_range(10.0..25.0);
                Boid {
                    x: rng.random_range(0.0..width as f64),
                    y: rng.random_range(0.0..height as f64),
                    vx: angle.cos() * speed,
                    vy: angle.sin() * speed,
                    hue: rng.random_range(0.0..1.0),
                }
            })
            .collect();

        Boids {
            width,
            height,
            boids,
        }
    }
}

impl Animation for Boids {
    fn name(&self) -> &str {
        "boids"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let visual_range = 25.0;
        let protected_range = 5.0;
        let max_speed = 35.0;
        let min_speed = 10.0;

        // Collect positions for rule calculations
        let positions: Vec<(f64, f64, f64, f64)> =
            self.boids.iter().map(|b| (b.x, b.y, b.vx, b.vy)).collect();

        for (i, boid) in self.boids.iter_mut().enumerate() {
            let mut sep_x = 0.0;
            let mut sep_y = 0.0;
            let mut align_x = 0.0;
            let mut align_y = 0.0;
            let mut cohes_x = 0.0;
            let mut cohes_y = 0.0;
            let mut neighbors = 0;

            for (j, &(ox, oy, ovx, ovy)) in positions.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dx = ox - boid.x;
                let dy = oy - boid.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < protected_range {
                    // Separation
                    sep_x -= dx / dist.max(0.1);
                    sep_y -= dy / dist.max(0.1);
                } else if dist < visual_range {
                    // Alignment
                    align_x += ovx;
                    align_y += ovy;
                    // Cohesion
                    cohes_x += ox;
                    cohes_y += oy;
                    neighbors += 1;
                }
            }

            if neighbors > 0 {
                let n = neighbors as f64;
                align_x /= n;
                align_y /= n;
                cohes_x = cohes_x / n - boid.x;
                cohes_y = cohes_y / n - boid.y;
            }

            // Apply forces
            let sep_factor = 2.0;
            let align_factor = 0.05;
            let cohes_factor = 0.005;

            boid.vx += sep_x * sep_factor + align_x * align_factor + cohes_x * cohes_factor;
            boid.vy += sep_y * sep_factor + align_y * align_factor + cohes_y * cohes_factor;

            // Edge avoidance
            let margin = 10.0;
            let turn_force = 3.0;
            if boid.x < margin {
                boid.vx += turn_force;
            }
            if boid.x > self.width as f64 - margin {
                boid.vx -= turn_force;
            }
            if boid.y < margin {
                boid.vy += turn_force;
            }
            if boid.y > self.height as f64 - margin {
                boid.vy -= turn_force;
            }

            // Speed limits
            let speed = (boid.vx * boid.vx + boid.vy * boid.vy).sqrt();
            if speed > max_speed {
                boid.vx = boid.vx / speed * max_speed;
                boid.vy = boid.vy / speed * max_speed;
            } else if speed < min_speed && speed > 0.01 {
                boid.vx = boid.vx / speed * min_speed;
                boid.vy = boid.vy / speed * min_speed;
            }

            boid.x += boid.vx * dt;
            boid.y += boid.vy * dt;

            // Wrap
            if boid.x < 0.0 {
                boid.x += self.width as f64;
            }
            if boid.x >= self.width as f64 {
                boid.x -= self.width as f64;
            }
            if boid.y < 0.0 {
                boid.y += self.height as f64;
            }
            if boid.y >= self.height as f64 {
                boid.y -= self.height as f64;
            }

            // Update hue based on heading
            let heading = boid.vy.atan2(boid.vx);
            boid.hue = (heading / std::f64::consts::TAU + 0.5).fract();
        }

        // Draw
        canvas.clear();
        for boid in &self.boids {
            let ix = boid.x as usize;
            let iy = boid.y as usize;
            if ix < canvas.width && iy < canvas.height {
                let (r, g, b) = hsv_to_rgb(boid.hue, 0.9, 1.0);
                canvas.set_colored(ix, iy, 1.0, r, g, b);

                // Draw small trail
                let speed = (boid.vx * boid.vx + boid.vy * boid.vy).sqrt();
                let norm_vx = if speed > 0.01 { boid.vx / speed } else { 0.0 };
                let norm_vy = if speed > 0.01 { boid.vy / speed } else { 0.0 };
                for t in 1..3 {
                    let tx = (boid.x - norm_vx * t as f64) as usize;
                    let ty = (boid.y - norm_vy * t as f64) as usize;
                    if tx < canvas.width && ty < canvas.height {
                        let fade = 1.0 - t as f64 * 0.35;
                        canvas.set_colored(tx, ty, fade, r, g, b);
                    }
                }
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
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
