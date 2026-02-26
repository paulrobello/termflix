use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Petal {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    spin: f64,
    spin_speed: f64,
    size: f64,
    shade: f64,
}

/// Cherry blossom petals drifting and spinning in wind
pub struct Petals {
    width: usize,
    height: usize,
    petals: Vec<Petal>,
    wind: f64,
    wind_target: f64,
    wind_timer: f64,
    rng: rand::rngs::ThreadRng,
}

impl Petals {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let count = ((width * height) as f64 / 150.0 * scale) as usize;
        let petals = (0..count)
            .map(|_| Petal {
                x: rng.random_range(0.0..width as f64),
                y: rng.random_range(-(height as f64 * 2.0)..(height as f64)),
                vx: rng.random_range(-2.0..2.0),
                vy: rng.random_range(3.0..8.0),
                spin: rng.random_range(0.0..std::f64::consts::TAU),
                spin_speed: rng.random_range(-3.0..3.0),
                size: rng.random_range(0.4..1.0),
                shade: rng.random_range(0.0..1.0),
            })
            .collect();
        Petals {
            width,
            height,
            petals,
            wind: 2.0,
            wind_target: 2.0,
            wind_timer: 0.0,
            rng: rand::rng(),
        }
    }
}

impl Animation for Petals {
    fn name(&self) -> &str {
        "petals"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;
        let w = self.width as f64;
        let h = self.height as f64;

        // Vary wind
        self.wind_timer -= dt;
        if self.wind_timer <= 0.0 {
            self.wind_target = self.rng.random_range(-3.0..5.0);
            self.wind_timer = self.rng.random_range(2.0..5.0);
        }
        self.wind += (self.wind_target - self.wind) * dt * 0.5;

        canvas.clear();

        for petal in &mut self.petals {
            // Spin
            petal.spin += petal.spin_speed * dt;

            // Wind and gravity
            let wobble = (time * 1.5 + petal.x * 0.05).sin() * 2.0;
            petal.vx += (self.wind + wobble - petal.vx) * dt * 0.5;
            petal.vy += (5.0 - petal.vy) * dt * 0.3;

            // Flutter effect based on spin
            let flutter = petal.spin.sin() * 1.5;
            petal.vx += flutter * dt;

            petal.x += petal.vx * dt;
            petal.y += petal.vy * dt;

            // Reset when off screen
            if petal.y > h + 5.0 || petal.x > w + 10.0 || petal.x < -10.0 {
                petal.x = self.rng.random_range(0.0..w);
                petal.y = self.rng.random_range(-20.0..-2.0);
                petal.vy = self.rng.random_range(3.0..8.0);
                petal.vx = self.rng.random_range(-2.0..2.0);
                petal.spin_speed = self.rng.random_range(-3.0..3.0);
            }

            // Draw petal as a small cluster based on spin angle
            let apparent_width = petal.spin.cos().abs() * petal.size;
            let brightness = 0.6 + apparent_width * 0.4;

            let (r, g, b) = petal_color(petal.shade);

            let px = petal.x as usize;
            let py = petal.y as usize;
            if px < canvas.width && py < canvas.height {
                canvas.set_colored(px, py, brightness, r, g, b);
            }

            // Draw second pixel for wider petals
            if apparent_width > 0.5 {
                let px2 = (petal.x + petal.spin.cos()) as usize;
                let py2 = (petal.y + petal.spin.sin() * 0.5) as usize;
                if px2 < canvas.width && py2 < canvas.height {
                    canvas.set_colored(px2, py2, brightness * 0.7, r, g, b);
                }
            }
        }
    }
}

fn petal_color(shade: f64) -> (u8, u8, u8) {
    if shade < 0.3 {
        // White petal
        (255, 240, 245)
    } else if shade < 0.6 {
        // Light pink
        (255, 182, 193)
    } else if shade < 0.85 {
        // Medium pink
        (255, 150, 170)
    } else {
        // Deep pink
        (238, 130, 160)
    }
}
