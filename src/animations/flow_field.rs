use super::Animation;
use crate::render::Canvas;
use noise::{NoiseFn, Perlin};
use rand::RngExt;

struct FlowParticle {
    x: f64,
    y: f64,
    prev_x: f64,
    prev_y: f64,
    speed: f64,
}

/// Perlin noise flow field visualization
pub struct FlowField {
    width: usize,
    height: usize,
    particles: Vec<FlowParticle>,
    noise: Perlin,
    trail: Vec<f64>,
    trail_colors: Vec<(u8, u8, u8)>,
}

impl FlowField {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let num_particles = ((width * height) as f64 / 60.0 * scale) as usize;
        let particles = (0..num_particles)
            .map(|_| {
                let x = rng.random_range(0.0..width as f64);
                let y = rng.random_range(0.0..height as f64);
                FlowParticle {
                    x,
                    y,
                    prev_x: x,
                    prev_y: y,
                    speed: rng.random_range(15.0..35.0),
                }
            })
            .collect();

        let size = width * height;
        FlowField {
            width,
            height,
            particles,
            noise: Perlin::new(rng.random_range(0..u32::MAX)),
            trail: vec![0.0; size],
            trail_colors: vec![(0, 0, 0); size],
        }
    }
}

impl Animation for FlowField {
    fn name(&self) -> &str {
        "flow"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let mut rng = rand::rng();

        // Fade trails
        for v in &mut self.trail {
            *v *= 0.97;
        }

        let scale = 0.01;
        let t = time * 0.3;

        for p in &mut self.particles {
            p.prev_x = p.x;
            p.prev_y = p.y;

            // Sample noise field for angle
            let angle = self.noise.get([p.x * scale, p.y * scale, t]) * std::f64::consts::TAU * 2.0;

            p.x += angle.cos() * p.speed * dt;
            p.y += angle.sin() * p.speed * dt;

            // Draw trail between prev and current position
            let ix = p.x as usize;
            let iy = p.y as usize;
            if ix < self.width && iy < self.height {
                let idx = iy * self.width + ix;
                self.trail[idx] = 1.0;

                // Color based on angle
                let hue = (angle + std::f64::consts::PI) / std::f64::consts::TAU;
                self.trail_colors[idx] = hue_to_rgb(hue);
            }

            // Wrap around edges
            if p.x < 0.0 || p.x >= self.width as f64 || p.y < 0.0 || p.y >= self.height as f64 {
                p.x = rng.random_range(0.0..self.width as f64);
                p.y = rng.random_range(0.0..self.height as f64);
                p.prev_x = p.x;
                p.prev_y = p.y;
            }
        }

        // Render trails to canvas
        canvas.clear();
        for y in 0..self.height.min(canvas.height) {
            for x in 0..self.width.min(canvas.width) {
                let idx = y * self.width + x;
                let v = self.trail[idx];
                if v > 0.05 {
                    let (r, g, b) = self.trail_colors[idx];
                    canvas.set_colored(x, y, v, r, g, b);
                }
            }
        }
    }
}

fn hue_to_rgb(h: f64) -> (u8, u8, u8) {
    let h = h.fract();
    let h = if h < 0.0 { h + 1.0 } else { h };
    let s = 1.0;
    let v = 1.0;
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
