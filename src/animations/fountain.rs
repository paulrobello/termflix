use super::Animation;
use crate::generators::{ColorGradient, ColorStop, EmitterConfig, ParticleSystem};
use crate::render::Canvas;
use rand::RngExt;

/// Water fountain shooting up from center bottom
pub struct Fountain {
    width: usize,
    height: usize,
    main_jet: ParticleSystem,
    splashes: ParticleSystem,
    mist: ParticleSystem,
    emit_accum: f64,
    rng: rand::rngs::ThreadRng,
}

impl Fountain {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let cx = width as f64 / 2.0;
        let bottom = height as f64 - 1.0;

        let jet_config = EmitterConfig {
            x: cx,
            y: bottom,
            spread: 0.4,
            angle: -std::f64::consts::FRAC_PI_2, // straight up
            speed_min: 35.0,
            speed_max: 55.0,
            life_min: 1.5,
            life_max: 3.0,
            gravity: 20.0,
            drag: 0.995,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 200,
                    g: 220,
                    b: 255,
                },
                ColorStop {
                    t: 0.3,
                    r: 100,
                    g: 160,
                    b: 255,
                },
                ColorStop {
                    t: 0.7,
                    r: 60,
                    g: 120,
                    b: 220,
                },
                ColorStop {
                    t: 1.0,
                    r: 30,
                    g: 60,
                    b: 140,
                },
            ]),
        };

        let splash_config = EmitterConfig {
            x: cx,
            y: bottom,
            spread: std::f64::consts::PI,
            angle: -std::f64::consts::FRAC_PI_2,
            speed_min: 5.0,
            speed_max: 20.0,
            life_min: 0.2,
            life_max: 0.6,
            gravity: 30.0,
            drag: 0.96,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 180,
                    g: 210,
                    b: 255,
                },
                ColorStop {
                    t: 1.0,
                    r: 80,
                    g: 100,
                    b: 180,
                },
            ]),
        };

        let mist_config = EmitterConfig {
            x: cx,
            y: bottom - 5.0,
            spread: std::f64::consts::PI * 0.6,
            angle: -std::f64::consts::FRAC_PI_2,
            speed_min: 1.0,
            speed_max: 5.0,
            life_min: 0.5,
            life_max: 1.5,
            gravity: -2.0, // mist floats up
            drag: 0.98,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 150,
                    g: 180,
                    b: 220,
                },
                ColorStop {
                    t: 1.0,
                    r: 80,
                    g: 100,
                    b: 140,
                },
            ]),
        };

        Fountain {
            width,
            height,
            main_jet: ParticleSystem::new(jet_config, (3000.0 * scale) as usize),
            splashes: ParticleSystem::new(splash_config, (2000.0 * scale) as usize),
            mist: ParticleSystem::new(mist_config, (500.0 * scale) as usize),
            emit_accum: 0.0,
            rng: rand::rng(),
        }
    }
}

impl Animation for Fountain {
    fn name(&self) -> &str {
        "fountain"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;

        let cx = self.width as f64 / 2.0;
        let bottom = self.height as f64 - 1.0;

        // Update emitter position
        self.main_jet.config.x = cx;
        self.main_jet.config.y = bottom;

        // Vary the jet angle slightly for organic movement
        let sway = (time * 1.5).sin() * 0.15;
        self.main_jet.config.angle = -std::f64::consts::FRAC_PI_2 + sway;

        // Vary power
        let power = 0.8 + (time * 0.7).sin() * 0.2;
        self.main_jet.config.speed_min = 35.0 * power;
        self.main_jet.config.speed_max = 55.0 * power;

        // Emit water particles continuously
        self.emit_accum += dt;
        let emit_interval = 0.01;
        while self.emit_accum >= emit_interval {
            self.main_jet.emit(3);
            self.emit_accum -= emit_interval;
        }

        // Emit mist near the base
        self.mist.config.x = cx + self.rng.random_range(-5.0..5.0);
        self.mist.config.y = bottom - self.rng.random_range(0.0..10.0);
        if self.rng.random_range(0.0..1.0) < 0.3 {
            self.mist.emit(1);
        }

        // Check for particles hitting the ground → create splashes
        for p in &self.main_jet.particles {
            if p.y >= bottom - 1.0 && p.vy > 0.0 && p.life > 0.0 {
                self.splashes.config.x = p.x;
                self.splashes.config.y = bottom - 1.0;
                let splash_count = if p.vy > 20.0 { 3 } else { 1 };
                self.splashes.emit(splash_count);
            }
        }

        // Update physics
        self.main_jet.update(dt);
        self.splashes.update(dt);
        self.mist.update(dt);

        // Draw
        canvas.clear();
        self.mist.draw(canvas);
        self.main_jet.draw(canvas);
        self.splashes.draw(canvas);
    }
}
