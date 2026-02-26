use super::Animation;
use crate::render::Canvas;

struct Star {
    x: f64,
    y: f64,
    z: f64,
    speed: f64,
}

/// 3D starfield with depth parallax
pub struct Starfield {
    stars: Vec<Star>,
    rng: rand::rngs::ThreadRng,
}

impl Starfield {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let num_stars = ((width * height) as f64 / 30.0 * scale) as usize;
        let stars = (0..num_stars).map(|_| new_star(&mut rng, false)).collect();
        Starfield { stars, rng: rand::rng() }
    }
}

fn new_star(rng: &mut impl rand::RngExt, far: bool) -> Star {
    Star {
        x: rng.random_range(-0.5..0.5),
        y: rng.random_range(-0.5..0.5),
        z: if far {
            rng.random_range(0.5..1.0)
        } else {
            rng.random_range(0.01..1.0)
        },
        speed: rng.random_range(0.15..0.45),
    }
}

impl Animation for Starfield {
    fn name(&self) -> &str {
        "starfield"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        canvas.clear();
        let cx = canvas.width as f64 / 2.0;
        let cy = canvas.height as f64 / 2.0;

        for star in &mut self.stars {
            star.z -= star.speed * dt;

            // Project 3D → 2D
            let px = (star.x / star.z) * cx + cx;
            let py = (star.y / star.z) * cy + cy;

            let ix = px as isize;
            let iy = py as isize;

            // Respawn if off-screen or too close
            if star.z <= 0.005
                || ix < 0
                || iy < 0
                || ix >= canvas.width as isize
                || iy >= canvas.height as isize
            {
                *star = new_star(&mut self.rng, true);
                continue;
            }

            let ix = ix as usize;
            let iy = iy as usize;

            let brightness = (1.0 - star.z).clamp(0.0, 1.0);
            let b = (brightness * 255.0) as u8;

            // Draw star — brighter stars are bigger (2x2 for close ones)
            canvas.set_colored(ix, iy, brightness, b, b, b.saturating_add(50));

            if brightness > 0.7 {
                // Draw larger for close/bright stars
                if ix + 1 < canvas.width {
                    canvas.set_colored(ix + 1, iy, brightness * 0.6, b, b, b.saturating_add(50));
                }
                if iy + 1 < canvas.height {
                    canvas.set_colored(ix, iy + 1, brightness * 0.6, b, b, b.saturating_add(50));
                }
            }
        }
    }
}
