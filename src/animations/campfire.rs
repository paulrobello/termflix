use super::Animation;
use crate::generators::{ColorGradient, ColorStop, EmitterConfig, ParticleSystem};
use crate::render::Canvas;
use rand::RngExt;

/// Realistic campfire with rising ember sparks
pub struct Campfire {
    width: usize,
    height: usize,
    fire_buf: Vec<f64>,
    embers: ParticleSystem,
    rng: rand::rngs::ThreadRng,
}

impl Campfire {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let ember_config = EmitterConfig {
            x: width as f64 * 0.5,
            y: height as f64 * 0.7,
            spread: 0.4,
            angle: -std::f64::consts::FRAC_PI_2,
            speed_min: 8.0,
            speed_max: 25.0,
            life_min: 1.0,
            life_max: 3.5,
            gravity: -3.0,
            drag: 0.98,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 255,
                    g: 200,
                    b: 50,
                },
                ColorStop {
                    t: 0.3,
                    r: 255,
                    g: 120,
                    b: 0,
                },
                ColorStop {
                    t: 0.7,
                    r: 200,
                    g: 50,
                    b: 0,
                },
                ColorStop {
                    t: 1.0,
                    r: 80,
                    g: 20,
                    b: 0,
                },
            ]),
        };

        Campfire {
            width,
            height,
            fire_buf: vec![0.0; width * height],
            embers: ParticleSystem::new(ember_config, (500.0 * scale) as usize),
            rng: rand::rng(),
        }
    }
}

impl Animation for Campfire {
    fn name(&self) -> &str {
        "campfire"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;
        let w = self.width;
        let h = self.height;
        let cx = w as f64 * 0.5;
        let base_y = h as f64 * 0.75;

        self.fire_buf.resize(w * h, 0.0);

        // Seed fire at base
        let fire_width = 10.0;
        for x in 0..w {
            let fx = x as f64;
            let dist = (fx - cx).abs();
            if dist < fire_width {
                let intensity = 1.0 - dist / fire_width;
                let row = (base_y as usize).min(h - 1);
                self.fire_buf[row * w + x] = intensity * self.rng.random_range(0.7..1.0);
            }
        }

        // Propagate fire upward
        for y in 0..h.saturating_sub(1) {
            for x in 0..w {
                let src_x = (x as isize + self.rng.random_range(-1i32..=1) as isize)
                    .clamp(0, w as isize - 1) as usize;
                let src_y = (y + 1).min(h - 1);
                let decay = self.rng.random_range(0.03..0.08);
                let val = (self.fire_buf[src_y * w + src_x] - decay).max(0.0);
                self.fire_buf[y * w + x] = val;
            }
        }

        canvas.clear();

        // Draw fire
        for y in 0..h.min(canvas.height) {
            for x in 0..w.min(canvas.width) {
                let v = self.fire_buf[y * w + x];
                if v > 0.01 {
                    let (r, g, b) = campfire_color(v);
                    canvas.set_colored(x, y, v, r, g, b);
                }
            }
        }

        // Draw logs (positioned relative to fire base)
        let log_specs: [(f64, f64, f64); 3] = [
            (cx - 3.0, base_y, 12.0),
            (cx + 2.0, base_y + 1.0, 10.0),
            (cx, base_y + 2.0, 8.0),
        ];
        for &(lx, ly, length) in &log_specs {
            let start_x = (lx - length * 0.5).max(0.0) as usize;
            let end_x = (lx + length * 0.5).min(canvas.width as f64) as usize;
            let iy = ly.min(canvas.height as f64 - 1.0) as usize;
            for x in start_x..end_x {
                if x < canvas.width && iy < canvas.height {
                    let glow = ((time * 2.0 + x as f64 * 0.3).sin() * 0.1 + 0.1).max(0.0);
                    let r = (90.0 + glow * 100.0) as u8;
                    let g = (50.0 + glow * 40.0) as u8;
                    let b = 20;
                    canvas.set_colored(x, iy, 0.9, r, g, b);
                }
            }
        }

        // Emit embers
        self.embers.config.x = cx + self.rng.random_range(-3.0..3.0);
        self.embers.config.y = base_y - 5.0;
        self.embers.config.wind = (time * 0.5).sin() * 2.0;
        if self.rng.random_range(0.0..1.0) < 0.3 {
            self.embers.emit(self.rng.random_range(1..4));
        }

        self.embers.update(dt);
        self.embers.draw(canvas);
    }
}

fn campfire_color(v: f64) -> (u8, u8, u8) {
    if v > 0.8 {
        let f = (v - 0.8) / 0.2;
        (255, (230.0 + 25.0 * f) as u8, (100.0 + 155.0 * f) as u8)
    } else if v > 0.5 {
        let f = (v - 0.5) / 0.3;
        (255, (120.0 + 110.0 * f) as u8, (10.0 + 90.0 * f) as u8)
    } else if v > 0.2 {
        let f = (v - 0.2) / 0.3;
        ((150.0 + 105.0 * f) as u8, (20.0 + 100.0 * f) as u8, 0)
    } else {
        let f = v / 0.2;
        ((50.0 + 100.0 * f) as u8, (5.0 + 15.0 * f) as u8, 0)
    }
}
