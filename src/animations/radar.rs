use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Blip {
    x: f64,
    y: f64,
    life: f64,
    max_life: f64,
}

/// Rotating radar sweep line with random blips that fade
pub struct Radar {
    blips: Vec<Blip>,
    sweep_angle: f64,
    rng: rand::rngs::ThreadRng,
}

impl Radar {
    pub fn new() -> Self {
        Radar {
            blips: Vec::new(),
            sweep_angle: 0.0,
            rng: rand::rng(),
        }
    }
}

impl Animation for Radar {
    fn name(&self) -> &str {
        "radar"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let radius = (w.min(h) * 0.45).max(10.0);

        // Rotate sweep
        let prev_angle = self.sweep_angle;
        self.sweep_angle += dt * 2.0; // ~1 revolution per pi seconds
        if self.sweep_angle >= std::f64::consts::TAU {
            self.sweep_angle -= std::f64::consts::TAU;
        }

        // Spawn blips along sweep line
        if self.rng.random_range(0.0..1.0) < 0.15 {
            let dist = self.rng.random_range(radius * 0.15..radius * 0.9);
            let blip_angle = self.sweep_angle + self.rng.random_range(-0.05..0.05);
            self.blips.push(Blip {
                x: cx + blip_angle.cos() * dist,
                y: cy + blip_angle.sin() * dist,
                life: 4.0,
                max_life: 4.0,
            });
        }

        // Update blip lifetimes
        for blip in &mut self.blips {
            blip.life -= dt;
        }
        self.blips.retain(|b| b.life > 0.0);

        canvas.clear();

        // Draw radar circle rings
        for ring in 1..=4 {
            let r = radius * ring as f64 / 4.0;
            let steps = (r * 4.0) as usize;
            for i in 0..steps {
                let angle = std::f64::consts::TAU * i as f64 / steps as f64;
                let px = (cx + angle.cos() * r) as usize;
                let py = (cy + angle.sin() * r) as usize;
                if px < canvas.width && py < canvas.height {
                    canvas.set_colored(px, py, 0.15, 0, 100, 0);
                }
            }
        }

        // Draw cross hairs
        for i in 0..(radius as usize) {
            let t = i as f64;
            let positions = [
                ((cx + t) as usize, cy as usize),
                ((cx - t).max(0.0) as usize, cy as usize),
                (cx as usize, (cy + t) as usize),
                (cx as usize, (cy - t).max(0.0) as usize),
            ];
            for (px, py) in positions {
                if px < canvas.width && py < canvas.height {
                    canvas.set_colored(px, py, 0.1, 0, 80, 0);
                }
            }
        }

        // Draw sweep line with trail
        let trail_angle = 0.6; // radians of trailing glow
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > radius || dist < 2.0 {
                    continue;
                }

                let pixel_angle = dy.atan2(dx);
                let mut angle_diff = self.sweep_angle - pixel_angle;

                // Normalize to [0, TAU)
                while angle_diff < 0.0 {
                    angle_diff += std::f64::consts::TAU;
                }
                while angle_diff >= std::f64::consts::TAU {
                    angle_diff -= std::f64::consts::TAU;
                }

                if angle_diff < trail_angle {
                    let trail_frac = 1.0 - angle_diff / trail_angle;
                    let intensity = trail_frac.powi(2) * 0.8;
                    let g = (200.0 * intensity) as u8;
                    let r = (30.0 * intensity) as u8;
                    canvas.set_colored(x, y, intensity, r, g, 0);
                }
            }
        }

        // Draw sweep line itself
        let steps = radius as usize;
        for i in 0..steps {
            let t = i as f64;
            let px = (cx + self.sweep_angle.cos() * t) as usize;
            let py = (cy + self.sweep_angle.sin() * t) as usize;
            if px < canvas.width && py < canvas.height {
                canvas.set_colored(px, py, 0.9, 50, 255, 50);
            }
        }

        // Draw blips
        for blip in &self.blips {
            let age = 1.0 - blip.life / blip.max_life;
            let brightness = (1.0 - age).powi(2);
            let px = blip.x as usize;
            let py = blip.y as usize;
            if px < canvas.width && py < canvas.height && brightness > 0.01 {
                let g = (255.0 * brightness) as u8;
                let r = (100.0 * brightness) as u8;
                canvas.set_colored(px, py, brightness, r, g, 0);

                // Glow around blip
                for &(ox, oy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let gx = (px as i32 + ox) as usize;
                    let gy = (py as i32 + oy) as usize;
                    if gx < canvas.width && gy < canvas.height {
                        canvas.set_colored(gx, gy, brightness * 0.3, r / 2, g / 2, 0);
                    }
                }
            }
        }

        // Center dot
        let icx = cx as usize;
        let icy = cy as usize;
        if icx < canvas.width && icy < canvas.height {
            canvas.set_colored(icx, icy, 1.0, 100, 255, 100);
        }

        let _ = prev_angle;
    }
}
