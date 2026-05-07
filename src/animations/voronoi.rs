use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Seed {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    hue: f64,
}

/// Animated Voronoi diagram with drifting seed points, edge detection, and Lloyd relaxation
pub struct Voronoi {
    width: usize,
    height: usize,
    seeds: Vec<Seed>,
    rng: rand::rngs::ThreadRng,
    last_relax_time: f64,
}

impl Voronoi {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut voronoi = Voronoi {
            width,
            height,
            seeds: Vec::new(),
            rng: rand::rng(),
            last_relax_time: 0.0,
        };
        voronoi.init_seeds(scale);
        voronoi
    }

    fn init_seeds(&mut self, scale: f64) {
        let count = ((8.0 + 7.0 * scale).clamp(8.0, 15.0)) as usize;
        let w = self.width as f64;
        let h = self.height as f64;
        self.seeds = (0..count)
            .map(|i| Seed {
                x: self.rng.random_range(w * 0.05..w * 0.95),
                y: self.rng.random_range(h * 0.05..h * 0.95),
                vx: self.rng.random_range(-6.0..6.0),
                vy: self.rng.random_range(-6.0..6.0),
                hue: i as f64 / count as f64,
            })
            .collect();
    }
}

impl Animation for Voronoi {
    fn name(&self) -> &str {
        "voronoi"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.init_seeds(1.0);
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = self.width as f64;
        let h = self.height as f64;

        // Update seed positions: drift and bounce off edges
        for seed in &mut self.seeds {
            seed.x += seed.vx * dt;
            seed.y += seed.vy * dt;

            if seed.x < 0.0 {
                seed.x = 0.0;
                seed.vx = seed.vx.abs();
            } else if seed.x >= w {
                seed.x = w - 1.0;
                seed.vx = -seed.vx.abs();
            }
            if seed.y < 0.0 {
                seed.y = 0.0;
                seed.vy = seed.vy.abs();
            } else if seed.y >= h {
                seed.y = h - 1.0;
                seed.vy = -seed.vy.abs();
            }
        }

        // Lloyd relaxation: every ~7 seconds, nudge seeds toward their cell centroids
        if time - self.last_relax_time > 7.0 {
            self.last_relax_time = time;
            let seed_count = self.seeds.len();
            let mut sum_x = vec![0.0f64; seed_count];
            let mut sum_y = vec![0.0f64; seed_count];
            let mut counts = vec![0usize; seed_count];

            for y in 0..canvas.height {
                let fy = y as f64;
                for x in 0..canvas.width {
                    let fx = x as f64;
                    let mut best_idx = 0;
                    let mut best_dist = f64::MAX;
                    for (i, seed) in self.seeds.iter().enumerate() {
                        let dx = fx - seed.x;
                        let dy = fy - seed.y;
                        let d = dx * dx + dy * dy;
                        if d < best_dist {
                            best_dist = d;
                            best_idx = i;
                        }
                    }
                    sum_x[best_idx] += fx;
                    sum_y[best_idx] += fy;
                    counts[best_idx] += 1;
                }
            }

            // Gradually move each seed toward its centroid (lerp factor 0.15)
            for (i, seed) in self.seeds.iter_mut().enumerate() {
                if counts[i] > 0 {
                    let cx = sum_x[i] / counts[i] as f64;
                    let cy = sum_y[i] / counts[i] as f64;
                    seed.x += (cx - seed.x) * 0.15;
                    seed.y += (cy - seed.y) * 0.15;
                }
            }
        }

        canvas.clear();

        // Render Voronoi cells with edge detection
        let max_dist = (w * w + h * h).sqrt() * 0.5;

        for y in 0..canvas.height {
            let fy = y as f64;
            for x in 0..canvas.width {
                let fx = x as f64;

                // Find nearest and second-nearest seeds
                let mut best_dist = f64::MAX;
                let mut second_dist = f64::MAX;
                let mut best_idx = 0;

                for (i, seed) in self.seeds.iter().enumerate() {
                    let dx = fx - seed.x;
                    let dy = fy - seed.y;
                    let d = dx * dx + dy * dy;
                    if d < best_dist {
                        second_dist = best_dist;
                        best_dist = d;
                        best_idx = i;
                    } else if d < second_dist {
                        second_dist = d;
                    }
                }

                let best_dist_root = best_dist.sqrt();
                let second_dist_root = second_dist.sqrt();

                // Distance-based dimming: closer = brighter
                let dim = 1.0 - (best_dist_root / max_dist * 1.5).min(0.6);

                // Edge detection: brighten where nearest and second-nearest are nearly equidistant
                let edge_ratio = if second_dist_root > 0.0 {
                    best_dist_root / second_dist_root
                } else {
                    1.0
                };
                let edge_factor = if edge_ratio > 0.85 { 1.0 } else { 0.0 };

                let brightness = if edge_factor > 0.0 {
                    // Bright white edge highlight
                    0.95
                } else {
                    dim * 0.85 + 0.15
                };

                let hue = self.seeds[best_idx].hue;
                let (r, g, b) = if edge_factor > 0.0 {
                    // Edge: bright white-tinted version of the cell color
                    let (cr, cg, cb) = hsv_to_rgb(hue, 0.3, 1.0);
                    (
                        (cr as f64 * 0.4 + 255.0 * 0.6) as u8,
                        (cg as f64 * 0.4 + 255.0 * 0.6) as u8,
                        (cb as f64 * 0.4 + 255.0 * 0.6) as u8,
                    )
                } else {
                    hsv_to_rgb(hue, 0.7, dim)
                };

                canvas.set_colored(x, y, brightness, r, g, b);
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = h.fract();
    let h = if h < 0.0 { h + 1.0 } else { h };
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
