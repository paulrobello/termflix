use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    max_life: f64,
    r: u8,
    g: u8,
    b: u8,
}

/// Fireworks / particle fountain
pub struct Particles {
    width: usize,
    height: usize,
    particles: Vec<Particle>,
    spawn_timer: f64,
    rng: rand::rngs::ThreadRng,
}

impl Particles {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        Particles {
            width,
            height,
            particles: Vec::with_capacity((2000.0 * scale) as usize),
            spawn_timer: 0.0,
            rng: rand::rng(),
        }
    }

    fn spawn_firework(&mut self) {
        let cx = self
            .rng
            .random_range(self.width as f64 * 0.2..self.width as f64 * 0.8);
        let cy = self
            .rng
            .random_range(self.height as f64 * 0.2..self.height as f64 * 0.6);
        let count = self.rng.random_range(30..80);
        let r: u8 = self.rng.random_range(100..255);
        let g: u8 = self.rng.random_range(100..255);
        let b: u8 = self.rng.random_range(100..255);

        for _ in 0..count {
            let angle = self.rng.random_range(0.0..std::f64::consts::TAU);
            let speed = self.rng.random_range(5.0..40.0);
            let life = self.rng.random_range(0.8..2.5);
            self.particles.push(Particle {
                x: cx,
                y: cy,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                life,
                max_life: life,
                r,
                g,
                b,
            });
        }
    }
}

impl Animation for Particles {
    fn name(&self) -> &str {
        "particles"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Spawn
        self.spawn_timer += dt;
        if self.spawn_timer > 0.8 {
            self.spawn_timer = 0.0;
            self.spawn_firework();
        }

        // Update
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 15.0 * dt; // gravity
            p.vx *= 0.99; // drag
            p.life -= dt;
        }

        // Remove dead
        self.particles.retain(|p| p.life > 0.0);

        // Draw
        canvas.clear();
        for p in &self.particles {
            let ix = p.x as usize;
            let iy = p.y as usize;
            if ix < canvas.width && iy < canvas.height {
                let fade = (p.life / p.max_life).clamp(0.0, 1.0);
                let r = (p.r as f64 * fade) as u8;
                let g = (p.g as f64 * fade) as u8;
                let b = (p.b as f64 * fade) as u8;
                canvas.set_colored(ix, iy, fade, r, g, b);
            }
        }
    }
}
