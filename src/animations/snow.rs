use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Snowflake {
    x: f64,
    y: f64,
    speed: f64,
    wobble_phase: f64,
    wobble_amp: f64,
    size: f64,
}

/// Snow falling and accumulating on the ground
pub struct Snow {
    width: usize,
    height: usize,
    flakes: Vec<Snowflake>,
    accumulation: Vec<f64>, // height of snow per column
    rng: rand::rngs::ThreadRng,
}

impl Snow {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let num_flakes = ((width * height) as f64 / 100.0 * scale) as usize;
        let flakes = (0..num_flakes)
            .map(|_| Snowflake {
                x: rng.random_range(0.0..width as f64),
                y: rng.random_range(-(height as f64)..height as f64),
                speed: rng.random_range(5.0..15.0),
                wobble_phase: rng.random_range(0.0..std::f64::consts::TAU),
                wobble_amp: rng.random_range(0.5..2.0),
                size: rng.random_range(0.5..1.5),
            })
            .collect();

        Snow {
            width,
            height,
            flakes,
            accumulation: vec![0.0; width],
            rng: rand::rng(),
        }
    }
}

impl Animation for Snow {
    fn name(&self) -> &str {
        "snow"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;

        // Resize accumulation if needed
        if self.accumulation.len() != self.width {
            self.accumulation.resize(self.width, 0.0);
        }

        canvas.clear();

        // Global wind
        let wind = (time * 0.3).sin() * 3.0;

        // Update and draw snowflakes
        for flake in &mut self.flakes {
            flake.y += flake.speed * dt;
            flake.x += (wind + (time * 2.0 + flake.wobble_phase).sin() * flake.wobble_amp) * dt;

            // Wrap horizontally
            if flake.x < 0.0 {
                flake.x += self.width as f64;
            }
            if flake.x >= self.width as f64 {
                flake.x -= self.width as f64;
            }

            let col = (flake.x as usize).min(self.width.saturating_sub(1));
            let ground_level = self.height as f64 - self.accumulation[col];

            // Check if flake hit the accumulated snow
            if flake.y >= ground_level - 1.0 {
                // Accumulate
                self.accumulation[col] += flake.size * 0.3;

                // Smooth accumulation with neighbors
                if col > 0 && self.accumulation[col] > self.accumulation[col - 1] + 2.0 {
                    self.accumulation[col] -= 0.5;
                    self.accumulation[col - 1] += 0.5;
                }
                if col + 1 < self.width && self.accumulation[col] > self.accumulation[col + 1] + 2.0
                {
                    self.accumulation[col] -= 0.5;
                    self.accumulation[col + 1] += 0.5;
                }

                // Cap accumulation
                if self.accumulation[col] > self.height as f64 * 0.6 {
                    self.accumulation[col] = self.height as f64 * 0.6;
                }

                // Reset flake at top
                flake.y = self.rng.random_range(-(self.height as f64 * 0.3)..0.0);
                flake.x = self.rng.random_range(0.0..self.width as f64);
                flake.speed = self.rng.random_range(5.0..15.0);
                continue;
            }

            // Draw snowflake
            let ix = flake.x as usize;
            let iy = flake.y as usize;
            if ix < canvas.width && iy < canvas.height {
                let brightness = 0.7 + flake.size * 0.2;
                canvas.set_colored(ix, iy, brightness, 230, 235, 255);
            }
        }

        // Draw accumulated snow
        for x in 0..self.width.min(canvas.width) {
            let snow_height = self.accumulation[x] as usize;
            let ground = self.height;
            for dy in 0..snow_height {
                let y = ground.saturating_sub(1 + dy);
                if y < canvas.height {
                    let depth = dy as f64 / snow_height as f64;
                    let r = (220.0 + 35.0 * (1.0 - depth)) as u8;
                    let g = (225.0 + 30.0 * (1.0 - depth)) as u8;
                    canvas.set_colored(x, y, 0.8 + 0.2 * (1.0 - depth), r, g, 255);
                }
            }
        }
    }
}
