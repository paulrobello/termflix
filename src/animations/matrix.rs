use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Drop {
    x: usize,
    y: f64,
    speed: f64,
    length: usize,
}

/// Matrix digital rain
pub struct Matrix {
    width: usize,
    height: usize,
    drops: Vec<Drop>,
}

impl Matrix {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let num_drops = ((width as f64 / 2.0) * scale) as usize;
        let drops = (0..num_drops)
            .map(|_| Drop {
                x: rng.random_range(0..width),
                y: rng.random_range(0.0..height as f64),
                speed: rng.random_range(4.0..20.0),
                length: rng.random_range(5..height / 2),
            })
            .collect();
        Matrix {
            width,
            height,
            drops,
        }
    }
}

impl Animation for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Ascii
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let mut rng = rand::rng();
        canvas.clear();

        for drop in &mut self.drops {
            drop.y += drop.speed * dt;

            // Reset drop when it falls off screen
            if drop.y as usize > self.height + drop.length {
                drop.y = -(drop.length as f64);
                drop.x = rng.random_range(0..self.width);
                drop.speed = rng.random_range(4.0..20.0);
                drop.length = rng.random_range(5..self.height / 2);
            }

            let head = drop.y as isize;
            for i in 0..drop.length {
                let py = head - i as isize;
                if py >= 0 && (py as usize) < canvas.height && drop.x < canvas.width {
                    let fade = 1.0 - (i as f64 / drop.length as f64);
                    let brightness = fade * fade; // quadratic falloff
                    let g = 100 + (155.0 * fade) as u8;
                    canvas.set_colored(drop.x, py as usize, brightness, 0, g, 0);
                }
            }
            // Bright white head
            if head >= 0 && (head as usize) < canvas.height && drop.x < canvas.width {
                canvas.set_colored(drop.x, head as usize, 1.0, 200, 255, 200);
            }
        }
    }
}
