use super::Animation;
use crate::generators::{ColorGradient, ColorStop, EmitterConfig, ParticleSystem};
use crate::render::Canvas;
use noise::{NoiseFn, Perlin};
use rand::RngExt;

/// Smoke rising with turbulence
pub struct Smoke {
    width: usize,
    height: usize,
    system: ParticleSystem,
    noise: Perlin,
    emit_accum: f64,
    rng: rand::rngs::ThreadRng,
}

impl Smoke {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let cx = width as f64 / 2.0;
        let bottom = height as f64 - 1.0;

        let config = EmitterConfig {
            x: cx,
            y: bottom,
            spread: 0.3,
            angle: -std::f64::consts::FRAC_PI_2,
            speed_min: 8.0,
            speed_max: 18.0,
            life_min: 3.0,
            life_max: 6.0,
            gravity: -3.0, // rises
            drag: 0.99,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 200,
                    g: 180,
                    b: 150,
                },
                ColorStop {
                    t: 0.3,
                    r: 150,
                    g: 140,
                    b: 130,
                },
                ColorStop {
                    t: 0.6,
                    r: 100,
                    g: 95,
                    b: 90,
                },
                ColorStop {
                    t: 1.0,
                    r: 50,
                    g: 48,
                    b: 46,
                },
            ]),
        };

        Smoke {
            width,
            height,
            system: ParticleSystem::new(config, (4000.0 * scale) as usize),
            noise: Perlin::new(123),
            emit_accum: 0.0,
            rng: rand::rng(),
        }
    }
}

impl Animation for Smoke {
    fn name(&self) -> &str {
        "smoke"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let cx = self.width as f64 / 2.0;
        let bottom = self.height as f64 - 1.0;

        self.system.config.x = cx + self.rng.random_range(-3.0..3.0);
        self.system.config.y = bottom;

        // Emit smoke
        self.emit_accum += dt;
        while self.emit_accum >= 0.02 {
            self.system.emit(2);
            self.emit_accum -= 0.02;
        }

        // Apply turbulence via noise before standard physics
        let t = time * 0.5;
        for p in &mut self.system.particles {
            let age = p.age();
            let turb_strength = 15.0 + age * 25.0; // more turbulence as smoke rises
            let nx = self.noise.get([p.x * 0.02, p.y * 0.02, t]);
            let ny = self.noise.get([p.x * 0.02 + 100.0, p.y * 0.02 + 100.0, t]);
            p.vx += nx * turb_strength * dt;
            p.vy += ny * turb_strength * dt * 0.5;
        }

        self.system.update(dt);

        // Draw with size based on age (smoke expands)
        canvas.clear();
        for p in &self.system.particles {
            let age = p.age();
            let size = (1.0 + age * 3.0) as usize;
            let (r, g, b) = self.system.config.gradient.sample(age);
            let brightness = p.life_frac() * 0.7;

            for dy in 0..size {
                for dx in 0..size {
                    let px = (p.x as isize + dx as isize - size as isize / 2) as usize;
                    let py = (p.y as isize + dy as isize - size as isize / 2) as usize;
                    if px < canvas.width && py < canvas.height {
                        let dist = (((dx as f64 - size as f64 / 2.0).powi(2)
                            + (dy as f64 - size as f64 / 2.0).powi(2))
                        .sqrt()
                            / (size as f64 / 2.0))
                            .clamp(0.0, 1.0);
                        let b_val = brightness * (1.0 - dist);
                        if b_val > 0.05 {
                            let idx = py * canvas.width + px;
                            let existing = canvas.pixels[idx];
                            if b_val > existing {
                                canvas.set_colored(px, py, b_val, r, g, b);
                            }
                        }
                    }
                }
            }
        }
    }
}
