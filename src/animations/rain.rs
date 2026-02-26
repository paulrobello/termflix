use super::Animation;
use crate::generators::{ColorGradient, ColorStop, EmitterConfig, ParticleSystem};
use crate::render::Canvas;
use rand::RngExt;

struct Raindrop {
    x: f64,
    y: f64,
    speed: f64,
    length: f64,
    wind_offset: f64,
    depth: f64, // 0.0 = far back, 1.0 = foreground
    r: u8,      // base color precomputed from depth
    g: u8,
    b: u8,
}

/// Rain with splash particles on impact
pub struct Rain {
    width: usize,
    height: usize,
    drops: Vec<Raindrop>,
    splashes: ParticleSystem,
    wind: f64,
    wind_target: f64,
    wind_timer: f64,
}

impl Rain {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let num_drops = ((width * height) as f64 / 80.0 * scale) as usize;
        let drops = (0..num_drops)
            .map(|_| {
                let depth = rng.random_range(0.0..1.0);
                let r = (60.0 + 80.0 * depth) as u8;
                let g = (80.0 + 90.0 * depth) as u8;
                let b = (120.0 + 135.0 * depth) as u8;
                Raindrop {
                    x: rng.random_range(0.0..width as f64),
                    y: rng.random_range(-(height as f64)..height as f64),
                    speed: 15.0 + depth * 50.0, // back: 15, front: 65
                    length: 1.0 + depth * 5.0,  // back: short, front: long
                    wind_offset: rng.random_range(-0.5..0.5),
                    depth,
                    r,
                    g,
                    b,
                }
            })
            .collect();

        let splash_config = EmitterConfig {
            x: 0.0,
            y: 0.0,
            spread: std::f64::consts::PI * 0.8,
            angle: -std::f64::consts::FRAC_PI_2, // upward
            speed_min: 10.0,
            speed_max: 35.0,
            life_min: 0.3,
            life_max: 0.8,
            gravity: 25.0,
            drag: 0.98,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 200,
                    g: 220,
                    b: 255,
                },
                ColorStop {
                    t: 0.5,
                    r: 150,
                    g: 180,
                    b: 255,
                },
                ColorStop {
                    t: 1.0,
                    r: 80,
                    g: 120,
                    b: 200,
                },
            ]),
        };

        Rain {
            width,
            height,
            drops,
            splashes: ParticleSystem::new(splash_config, (2000.0 * scale) as usize),
            wind: 0.0,
            wind_target: 0.0,
            wind_timer: 0.0,
        }
    }
}

impl Animation for Rain {
    fn name(&self) -> &str {
        "rain"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::HalfBlock
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;

        // Vary wind over time
        self.wind_timer -= dt;
        if self.wind_timer <= 0.0 {
            self.wind_target = rng.random_range(-8.0..8.0);
            self.wind_timer = rng.random_range(2.0..6.0);
        }
        self.wind += (self.wind_target - self.wind) * dt * 0.5;

        canvas.clear();

        // Update and draw raindrops
        for drop in &mut self.drops {
            let effective_wind = self.wind + drop.wind_offset;
            drop.x += effective_wind * dt;
            drop.y += drop.speed * dt;

            // Draw raindrop — depth affects brightness and color
            let depth_brightness = 0.25 + drop.depth * 0.75; // back: dim, front: bright
            let steps = drop.length as usize;
            for i in 0..steps {
                let t = i as f64 / drop.length.max(1.0);
                let px = (drop.x - effective_wind * t * 0.1) as usize;
                let py = (drop.y - t * drop.length * 0.5) as usize;
                if py < canvas.height {
                    let brightness = depth_brightness * (0.5 + 0.5 * (1.0 - t));
                    canvas.set_colored(px, py, brightness, drop.r, drop.g, drop.b);
                }
            }

            // Splash on ground impact — only foreground drops splash visibly
            if drop.y >= self.height as f64 - 1.0 {
                if drop.depth > 0.4 {
                    let splash_count = (drop.length as usize * 2).clamp(4, 10);
                    self.splashes.config.x = drop.x;
                    self.splashes.config.y = self.height as f64 - 2.0;
                    self.splashes.config.wind = self.wind * 0.5;
                    self.splashes.emit(splash_count);
                }

                // Reset drop at top, keep same depth layer
                // depth (and thus r/g/b) preserved across resets
                drop.y = rng.random_range(-(self.height as f64 * 0.3)..0.0);
                drop.x = rng.random_range(0.0..self.width as f64);
                drop.speed = 15.0 + drop.depth * 50.0;
                drop.length = 1.0 + drop.depth * 5.0;
                drop.wind_offset = rng.random_range(-0.5..0.5);
            }

            // Wrap horizontally
            if drop.x < 0.0 {
                drop.x += self.width as f64;
            } else if drop.x >= self.width as f64 {
                drop.x -= self.width as f64;
            }
        }

        // Update and draw splashes
        self.splashes.update(dt);
        self.splashes.draw(canvas);
    }
}
