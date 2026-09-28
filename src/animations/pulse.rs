use super::Animation;
use crate::color::hsv_to_rgb;
use crate::render::Canvas;

/// Expanding pulse rings from center
pub struct Pulse {
    rings: Vec<PulseRing>,
    spawn_timer: f64,
}

struct PulseRing {
    radius: f64,
    max_radius: f64,
    speed: f64,
    hue: f64,
}

impl Pulse {
    #[allow(unused_variables)]
    pub fn new(_width: usize, _height: usize, _scale: f64) -> Self {
        Pulse {
            rings: Vec::new(),
            spawn_timer: 0.0,
        }
    }
}

impl Animation for Pulse {
    fn name(&self) -> &str {
        "pulse"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let max_r = (cx * cx + cy * cy).sqrt();

        // Spawn new rings
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            self.rings.push(PulseRing {
                radius: 0.0,
                max_radius: max_r,
                speed: 30.0 + (time * 0.5).sin() * 10.0,
                hue: (time * 0.15).fract(),
            });
            self.spawn_timer = 0.5 + (time * 0.3).sin().abs() * 0.5;
        }

        // Update rings
        for ring in &mut self.rings {
            ring.radius += ring.speed * dt;
        }
        self.rings.retain(|r| r.radius < r.max_radius);

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                let mut total_brightness = 0.0f64;
                let mut total_r = 0.0f64;
                let mut total_g = 0.0f64;
                let mut total_b = 0.0f64;

                for ring in &self.rings {
                    let ring_dist = (dist - ring.radius).abs();
                    let width = 3.0 + ring.radius * 0.05;

                    if ring_dist < width {
                        let fade = 1.0 - (ring.radius / ring.max_radius);
                        let edge = 1.0 - (ring_dist / width);
                        let brightness = edge * edge * fade;

                        if brightness > 0.01 {
                            let (r, g, b) = hsv_to_rgb(ring.hue, 0.8, 1.0);
                            total_brightness += brightness;
                            total_r += r as f64 * brightness;
                            total_g += g as f64 * brightness;
                            total_b += b as f64 * brightness;
                        }
                    }
                }

                if total_brightness > 0.05 {
                    let b = total_brightness.clamp(0.0, 1.0);
                    let r = (total_r / total_brightness).clamp(0.0, 255.0) as u8;
                    let g = (total_g / total_brightness).clamp(0.0, 255.0) as u8;
                    let bl = (total_b / total_brightness).clamp(0.0, 255.0) as u8;
                    canvas.set_colored(x, y, b, r, g, bl);
                }
            }
        }
    }
}
