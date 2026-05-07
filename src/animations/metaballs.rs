use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Ball {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    radius: f64,
    hue: f64,
}

/// Metaballs using signed distance fields with HSV color blending
pub struct Metaballs {
    width: usize,
    height: usize,
    balls: Vec<Ball>,
    rng: rand::rngs::ThreadRng,
}

impl Metaballs {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let count = ((4.0 + 2.0 * scale).clamp(4.0, 6.0)) as usize;
        let w = width as f64;
        let h = height as f64;
        let balls = (0..count)
            .map(|i| {
                let angle = rng.random_range(0.0..std::f64::consts::TAU);
                let speed = rng.random_range(8.0..20.0);
                Ball {
                    x: rng.random_range(0.0..w),
                    y: rng.random_range(0.0..h),
                    vx: angle.cos() * speed,
                    vy: angle.sin() * speed,
                    radius: rng.random_range(3.0..8.0),
                    hue: i as f64 / count as f64,
                }
            })
            .collect();
        Metaballs {
            width,
            height,
            balls,
            rng: rand::rng(),
        }
    }
}

impl Animation for Metaballs {
    fn name(&self) -> &str {
        "metaballs"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = self.width as f64;
        let h = self.height as f64;

        // Update ball positions with bouncing
        for ball in &mut self.balls {
            ball.x += ball.vx * dt;
            ball.y += ball.vy * dt;

            // Gentle hue drift
            ball.hue = (ball.hue + dt * 0.02) % 1.0;

            // Radius pulsing
            ball.radius = (ball.radius + self.rng.random_range(-0.3..0.3) * dt).clamp(3.0, 10.0);

            // Bounce off canvas edges
            if ball.x < ball.radius {
                ball.x = ball.radius;
                ball.vx = ball.vx.abs();
            }
            if ball.x > w - ball.radius {
                ball.x = w - ball.radius;
                ball.vx = -ball.vx.abs();
            }
            if ball.y < ball.radius {
                ball.y = ball.radius;
                ball.vy = ball.vy.abs();
            }
            if ball.y > h - ball.radius {
                ball.y = h - ball.radius;
                ball.vy = -ball.vy.abs();
            }
        }

        canvas.clear();

        // Render metaball field
        let threshold = 1.0;
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                // Compute field: sum of (radius^2 / distance^2) for each ball
                let mut field = 0.0;
                let mut weighted_r = 0.0;
                let mut weighted_g = 0.0;
                let mut weighted_b = 0.0;
                let mut total_weight = 0.0;

                for ball in &self.balls {
                    let dx = fx - ball.x;
                    let dy = fy - ball.y;
                    let dist_sq = dx * dx + dy * dy;
                    let contribution = (ball.radius * ball.radius) / (dist_sq + 1.0);
                    field += contribution;

                    // Weight color by contribution for blending
                    let (cr, cg, cb) = hsv_to_rgb(ball.hue, 0.85, 1.0);
                    weighted_r += cr as f64 * contribution;
                    weighted_g += cg as f64 * contribution;
                    weighted_b += cb as f64 * contribution;
                    total_weight += contribution;
                }

                if field > threshold * 0.5 {
                    // Smooth fade near boundary: full brightness above threshold,
                    // fading below for organic edges
                    let edge_start = threshold * 0.5;
                    let brightness = if field >= threshold {
                        1.0
                    } else {
                        ((field - edge_start) / (threshold - edge_start)).clamp(0.0, 1.0)
                    };

                    // Normalize blended color
                    let (r, g, b) = if total_weight > 0.0 {
                        let r = (weighted_r / total_weight).clamp(0.0, 255.0) as u8;
                        let g = (weighted_g / total_weight).clamp(0.0, 255.0) as u8;
                        let b = (weighted_b / total_weight).clamp(0.0, 255.0) as u8;
                        (r, g, b)
                    } else {
                        (255, 255, 255)
                    };

                    // Brighten core regions above threshold
                    let final_brightness = if field > threshold * 2.0 {
                        let glow = ((field - threshold * 2.0) / threshold).clamp(0.0, 1.0);
                        // Mix toward white in very high-field regions (center of merged balls)
                        let gr = r as f64 + (255.0 - r as f64) * glow * 0.5;
                        let gg = g as f64 + (255.0 - g as f64) * glow * 0.5;
                        let gb = b as f64 + (255.0 - b as f64) * glow * 0.5;
                        canvas.set_colored(
                            x,
                            y,
                            brightness,
                            gr.clamp(0.0, 255.0) as u8,
                            gg.clamp(0.0, 255.0) as u8,
                            gb.clamp(0.0, 255.0) as u8,
                        );
                        continue;
                    } else {
                        brightness
                    };

                    canvas.set_colored(x, y, final_brightness, r, g, b);
                }
            }
        }

        // Subtle time-based hue offset applied in next frame via ball.hue drift
        let _ = time;
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
