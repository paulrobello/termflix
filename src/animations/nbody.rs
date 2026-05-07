use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Maximum number of trail positions stored per body.
const MAX_TRAIL: usize = 30;

/// Gravitational constant tuned for terminal-scale canvas.
const G: f64 = 800.0;

/// Softening factor to prevent singularities when bodies get very close.
const SOFTENING: f64 = 4.0;

/// Minimum number of bodies to maintain; respawn new ones if count drops below this.
const MIN_BODIES: usize = 3;

#[derive(Clone)]
struct Body {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    mass: f64,
    hue: f64,
    trail: Vec<(f64, f64)>,
}

/// N-body gravitational simulation with colorful orbiting masses and fading trails.
pub struct NBody {
    width: usize,
    height: usize,
    bodies: Vec<Body>,
}

impl NBody {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let mut sim = NBody {
            width,
            height,
            bodies: Vec::new(),
        };
        sim.spawn_initial_bodies();
        sim
    }

    fn spawn_initial_bodies(&mut self) {
        let mut rng = rand::rng();
        let count = rng.random_range(5..=8) as usize;
        let cx = self.width as f64 * 0.5;
        let cy = self.height as f64 * 0.5;
        let spread = self.width.min(self.height) as f64 * 0.3;

        for _ in 0..count {
            let angle = rng.random_range(0.0..std::f64::consts::TAU);
            let dist = rng.random_range(spread * 0.2..spread);
            let x = cx + angle.cos() * dist;
            let y = cy + angle.sin() * dist;

            // Give tangential velocity for orbital motion
            let tangent = angle + std::f64::consts::FRAC_PI_2;
            let orbital_speed = rng.random_range(8.0..20.0);
            let vx = tangent.cos() * orbital_speed;
            let vy = tangent.sin() * orbital_speed;

            let mass = rng.random_range(1.0..8.0);
            let hue = rng.random_range(0.0..1.0);

            self.bodies.push(Body {
                x,
                y,
                vx,
                vy,
                mass,
                hue,
                trail: Vec::new(),
            });
        }
    }

    fn spawn_body(&mut self) {
        let mut rng = rand::rng();
        let angle = rng.random_range(0.0..std::f64::consts::TAU);
        let dist = rng.random_range(5.0..(self.width.min(self.height) as f64 * 0.4));
        let cx = self.width as f64 * 0.5;
        let cy = self.height as f64 * 0.5;
        let x = cx + angle.cos() * dist;
        let y = cy + angle.sin() * dist;

        let tangent = angle + std::f64::consts::FRAC_PI_2;
        let orbital_speed = rng.random_range(8.0..20.0);
        let vx = tangent.cos() * orbital_speed;
        let vy = tangent.sin() * orbital_speed;

        let mass = rng.random_range(1.0..6.0);
        let hue = rng.random_range(0.0..1.0);

        self.bodies.push(Body {
            x,
            y,
            vx,
            vy,
            mass,
            hue,
            trail: Vec::new(),
        });
    }

    fn physics_step(&mut self, dt: f64) {
        let n = self.bodies.len();
        if n == 0 {
            return;
        }

        // Compute accelerations from gravitational forces
        let mut ax = vec![0.0f64; n];
        let mut ay = vec![0.0f64; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.bodies[j].x - self.bodies[i].x;
                let dy = self.bodies[j].y - self.bodies[i].y;
                let dist_sq = dx * dx + dy * dy + SOFTENING * SOFTENING;
                let dist = dist_sq.sqrt();
                let force = G / dist_sq;

                let fx = force * dx / dist;
                let fy = force * dy / dist;

                ax[i] += fx * self.bodies[j].mass;
                ay[i] += fy * self.bodies[j].mass;
                ax[j] -= fx * self.bodies[i].mass;
                ay[j] -= fy * self.bodies[i].mass;
            }
        }

        // Euler integration: update velocities then positions
        let w = self.width as f64;
        let h = self.height as f64;

        for i in 0..n {
            self.bodies[i].vx += ax[i] * dt;
            self.bodies[i].vy += ay[i] * dt;
            self.bodies[i].x += self.bodies[i].vx * dt;
            self.bodies[i].y += self.bodies[i].vy * dt;

            // Soft bounce off edges
            let margin = 2.0;
            let damping = 0.6;
            if self.bodies[i].x < margin {
                self.bodies[i].x = margin;
                self.bodies[i].vx = self.bodies[i].vx.abs() * damping;
            } else if self.bodies[i].x > w - margin {
                self.bodies[i].x = w - margin;
                self.bodies[i].vx = -self.bodies[i].vx.abs() * damping;
            }
            if self.bodies[i].y < margin {
                self.bodies[i].y = margin;
                self.bodies[i].vy = self.bodies[i].vy.abs() * damping;
            } else if self.bodies[i].y > h - margin {
                self.bodies[i].y = h - margin;
                self.bodies[i].vy = -self.bodies[i].vy.abs() * damping;
            }

            // Record trail position
            let pos = (self.bodies[i].x, self.bodies[i].y);
            self.bodies[i].trail.push(pos);
            if self.bodies[i].trail.len() > MAX_TRAIL {
                self.bodies[i].trail.remove(0);
            }
        }
    }

    fn handle_collisions(&mut self) {
        let mut merged = vec![false; self.bodies.len()];
        let mut new_bodies = Vec::new();

        for i in 0..self.bodies.len() {
            if merged[i] {
                continue;
            }
            let mut bi = self.bodies[i].clone();
            for (j, body_j) in self.bodies.iter().enumerate().skip(i + 1) {
                if merged[j] {
                    continue;
                }
                let dx = body_j.x - bi.x;
                let dy = body_j.y - bi.y;
                let dist = (dx * dx + dy * dy).sqrt();
                // Collision threshold based on combined masses
                let threshold = (bi.mass + body_j.mass).sqrt() * 1.2;
                if dist < threshold {
                    let total_mass = bi.mass + body_j.mass;
                    // Weighted average position and velocity
                    bi.x = (bi.x * bi.mass + body_j.x * body_j.mass) / total_mass;
                    bi.y = (bi.y * bi.mass + body_j.y * body_j.mass) / total_mass;
                    bi.vx = (bi.vx * bi.mass + body_j.vx * body_j.mass) / total_mass;
                    bi.vy = (bi.vy * bi.mass + body_j.vy * body_j.mass) / total_mass;
                    // Keep the color of the larger body
                    if body_j.mass > bi.mass {
                        bi.hue = body_j.hue;
                    }
                    bi.mass = total_mass;
                    merged[j] = true;
                }
            }
            new_bodies.push(bi);
        }

        self.bodies = new_bodies;
    }
}

impl Animation for NBody {
    fn name(&self) -> &str {
        "nbody"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Sub-step physics for stability: 2 steps at half dt
        let physics_dt = dt * 0.5;
        for _ in 0..2 {
            self.physics_step(physics_dt);
        }

        // Handle collisions (merging)
        self.handle_collisions();

        // Respawn bodies if count drops too low
        while self.bodies.len() < MIN_BODIES {
            self.spawn_body();
        }

        // Draw
        canvas.clear();

        for body in &self.bodies {
            let (r, g, b) = hsv_to_rgb(body.hue, 0.85, 1.0);
            let trail_len = body.trail.len();

            // Draw trail: older positions are dimmer, newer are brighter
            for (ti, &(tx, ty)) in body.trail.iter().enumerate() {
                let ix = tx as usize;
                let iy = ty as usize;
                if ix < canvas.width && iy < canvas.height {
                    let t = (ti + 1) as f64 / trail_len as f64;
                    let brightness = t * 0.5;
                    let (tr, tg, tb) = hsv_to_rgb(body.hue, 0.6, 0.3 + t * 0.7);
                    canvas.set_colored(ix, iy, brightness, tr, tg, tb);
                }
            }

            // Draw the body itself as a bright pixel proportional to mass
            let ix = body.x as usize;
            let iy = body.y as usize;
            if ix < canvas.width && iy < canvas.height {
                let brightness = 0.7 + (body.mass / 20.0).min(0.3);
                canvas.set_colored(ix, iy, brightness, r, g, b);

                // Draw a glow around larger bodies
                let glow_radius = (body.mass / 3.0).ceil() as usize;
                if glow_radius > 0 {
                    for dy in -(glow_radius as i32)..=(glow_radius as i32) {
                        for dx in -(glow_radius as i32)..=(glow_radius as i32) {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let gx = (ix as i32 + dx) as usize;
                            let gy = (iy as i32 + dy) as usize;
                            if gx < canvas.width && gy < canvas.height {
                                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                                if dist <= glow_radius as f64 {
                                    let falloff = 1.0 - dist / glow_radius as f64;
                                    let glow_bright = falloff * 0.3;
                                    canvas.set_colored(gx, gy, glow_bright, r, g, b);
                                }
                            }
                        }
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
