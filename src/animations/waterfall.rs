use crate::generators::{ColorGradient, ColorStop, EmitterConfig, ParticleSystem};
use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

struct WaterDrop {
    x: f64,
    y: f64,
    vy: f64,
    brightness: f64,
}

/// Cascading water from top with mist spray at bottom
pub struct Waterfall {
    width: usize,
    height: usize,
    drops: Vec<WaterDrop>,
    mist: ParticleSystem,
    fall_x: f64,
    fall_width: f64,
}

impl Waterfall {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let fall_x = width as f64 * 0.5;
        let fall_width = (width as f64 * 0.25).max(8.0);
        let count = ((width * height) as f64 / 40.0 * scale) as usize;
        let mut rng = rand::rng();

        let drops = (0..count)
            .map(|_| WaterDrop {
                x: fall_x + rng.random_range(-fall_width * 0.5..fall_width * 0.5),
                y: rng.random_range(0.0..height as f64),
                vy: rng.random_range(15.0..35.0),
                brightness: rng.random_range(0.4..1.0),
            })
            .collect();

        let mist_config = EmitterConfig {
            x: fall_x,
            y: height as f64 * 0.9,
            spread: std::f64::consts::PI * 1.0,
            angle: -std::f64::consts::FRAC_PI_2,
            speed_min: 5.0,
            speed_max: 25.0,
            life_min: 0.8,
            life_max: 3.0,
            gravity: -2.0,
            drag: 0.95,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop { t: 0.0, r: 200, g: 220, b: 255 },
                ColorStop { t: 0.5, r: 150, g: 180, b: 220 },
                ColorStop { t: 1.0, r: 100, g: 130, b: 180 },
            ]),
        };

        Waterfall {
            width,
            height,
            drops,
            mist: ParticleSystem::new(mist_config, (1500.0 * scale) as usize),
            fall_x,
            fall_width,
        }
    }
}

impl Animation for Waterfall {
    fn name(&self) -> &str {
        "waterfall"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;
        let h = self.height as f64;
        self.fall_x = self.width as f64 * 0.5;
        self.fall_width = (self.width as f64 * 0.25).max(8.0);

        let pool_y = h * 0.85;

        canvas.clear();

        // Draw rock face behind waterfall
        let rock_left = (self.fall_x - self.fall_width * 0.7) as usize;
        let rock_right = (self.fall_x + self.fall_width * 0.7) as usize;
        for x in rock_left..rock_right.min(canvas.width) {
            for y in 0..(pool_y as usize).min(canvas.height) {
                let noise = ((x as f64 * 0.3 + y as f64 * 0.2).sin() * 0.1 + 0.1).max(0.0);
                let shade = 0.15 + noise;
                let r = (60.0 * shade / 0.25) as u8;
                let g = (65.0 * shade / 0.25) as u8;
                let b = (70.0 * shade / 0.25) as u8;
                canvas.set_colored(x, y, 0.3, r, g, b);
            }
        }

        // Update and draw water drops
        for drop in &mut self.drops {
            drop.vy += 20.0 * dt; // gravity acceleration
            drop.y += drop.vy * dt;

            // Add slight horizontal drift
            drop.x += (time * 2.0 + drop.y * 0.1).sin() * 0.3;

            if drop.y >= pool_y {
                // Splash at bottom
                drop.y = rng.random_range(-5.0..0.0);
                drop.x = self.fall_x + rng.random_range(-self.fall_width * 0.5..self.fall_width * 0.5);
                drop.vy = rng.random_range(15.0..35.0);
                drop.brightness = rng.random_range(0.4..1.0);
            }

            let px = drop.x as usize;
            let py = drop.y as usize;
            if px < canvas.width && py < canvas.height {
                let speed_factor = (drop.vy / 35.0).clamp(0.3, 1.0);
                let b_val = (180.0 + 75.0 * speed_factor) as u8;
                let g_val = (200.0 + 40.0 * speed_factor) as u8;
                canvas.set_colored(px, py, drop.brightness, 180, g_val, b_val);

                // Streak below for fast drops
                if drop.vy > 20.0 {
                    let streak_len = (drop.vy * 0.05) as usize;
                    for s in 1..streak_len.min(4) {
                        let sy = py.wrapping_sub(s);
                        if sy < canvas.height {
                            let fade = 1.0 - s as f64 / streak_len as f64;
                            canvas.set_colored(px, sy, drop.brightness * fade * 0.5, 160, 190, b_val);
                        }
                    }
                }
            }
        }

        // Draw pool at bottom
        for y in (pool_y as usize)..canvas.height {
            for x in 0..canvas.width {
                let depth = (y as f64 - pool_y) / (h - pool_y);
                let wave = ((x as f64 * 0.2 + time * 3.0).sin() * 0.05).max(0.0);
                let brightness = 0.4 - depth * 0.2 + wave;
                let r = (40.0 - depth * 20.0) as u8;
                let g = (80.0 + depth * 30.0) as u8;
                let b = (160.0 + depth * 40.0).min(220.0) as u8;
                canvas.set_colored(x, y, brightness, r, g, b);
            }
        }

        // Emit mist at base
        self.mist.config.x = self.fall_x + rng.random_range(-self.fall_width * 0.5..self.fall_width * 0.5);
        self.mist.config.y = pool_y;
        self.mist.config.wind = (time * 0.3).sin() * 5.0;
        self.mist.emit(rng.random_range(4..10));
        self.mist.update(dt);
        self.mist.draw(canvas);
    }
}
