use crate::render::Canvas;
use super::Animation;

struct Electron {
    orbit_radius_x: f64,
    orbit_radius_y: f64,
    speed: f64,
    tilt: f64,
    phase: f64,
    hue: f64,
}

/// Electrons orbiting a nucleus in 3D perspective
pub struct Atom {
    electrons: Vec<Electron>,
}

impl Atom {
    pub fn new() -> Self {
        let orbits = vec![
            Electron {
                orbit_radius_x: 0.35,
                orbit_radius_y: 0.15,
                speed: 2.0,
                tilt: 0.0,
                phase: 0.0,
                hue: 0.0,
            },
            Electron {
                orbit_radius_x: 0.35,
                orbit_radius_y: 0.15,
                speed: 2.3,
                tilt: std::f64::consts::TAU / 3.0,
                phase: std::f64::consts::TAU / 3.0,
                hue: 0.33,
            },
            Electron {
                orbit_radius_x: 0.35,
                orbit_radius_y: 0.15,
                speed: 1.8,
                tilt: 2.0 * std::f64::consts::TAU / 3.0,
                phase: 2.0 * std::f64::consts::TAU / 3.0,
                hue: 0.66,
            },
        ];
        Atom { electrons: orbits }
    }
}

impl Animation for Atom {
    fn name(&self) -> &str {
        "atom"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;

        canvas.clear();

        // Draw orbit paths and electrons
        for electron in &self.electrons {
            let rx = w * electron.orbit_radius_x;
            let ry = h * electron.orbit_radius_y;
            let tilt = electron.tilt;
            let cos_tilt = tilt.cos();
            let sin_tilt = tilt.sin();

            // Draw orbit path
            let steps = (rx.max(ry) * 6.0) as usize;
            for i in 0..steps {
                let angle = std::f64::consts::TAU * i as f64 / steps as f64;
                let ox = angle.cos() * rx;
                let oy = angle.sin() * ry;

                // Rotate by tilt
                let rotated_x = ox * cos_tilt - oy * sin_tilt;
                let rotated_y = ox * sin_tilt + oy * cos_tilt;

                // 3D perspective: use rotated_y for depth
                let depth = (rotated_y / ry + 1.0) * 0.5;
                let brightness = 0.1 + depth * 0.1;

                let px = (cx + rotated_x) as usize;
                let py = (cy + rotated_y) as usize;
                if px < canvas.width && py < canvas.height {
                    let (r, g, b) = hsv_to_rgb(electron.hue, 0.3, 0.5);
                    canvas.set_colored(px, py, brightness, r, g, b);
                }
            }

            // Draw electron
            let angle = time * electron.speed + electron.phase;
            let ox = angle.cos() * rx;
            let oy = angle.sin() * ry;
            let rotated_x = ox * cos_tilt - oy * sin_tilt;
            let rotated_y = ox * sin_tilt + oy * cos_tilt;

            let ex = cx + rotated_x;
            let ey = cy + rotated_y;

            // Electron glow
            let glow_r = 3.0;
            for dy in -(glow_r as i32)..=(glow_r as i32) {
                for dx in -(glow_r as i32)..=(glow_r as i32) {
                    let dist = ((dx * dx + dy * dy) as f64).sqrt();
                    if dist <= glow_r {
                        let px = (ex + dx as f64) as usize;
                        let py = (ey + dy as f64) as usize;
                        if px < canvas.width && py < canvas.height {
                            let brightness = (1.0 - dist / glow_r).powi(2);
                            let (r, g, b) = hsv_to_rgb(electron.hue, 0.8, 1.0);
                            canvas.set_colored(px, py, brightness, r, g, b);
                        }
                    }
                }
            }
        }

        // Draw nucleus at center
        let nuc_r = (w.min(h) * 0.03).max(2.0);
        for dy in -(nuc_r as i32)..=(nuc_r as i32) {
            for dx in -(nuc_r as i32)..=(nuc_r as i32) {
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist <= nuc_r {
                    let px = (cx + dx as f64) as usize;
                    let py = (cy + dy as f64) as usize;
                    if px < canvas.width && py < canvas.height {
                        let brightness = (1.0 - dist / nuc_r).powi(2) * 0.9;
                        // Nucleus pulses
                        let pulse = (time * 3.0).sin() * 0.1 + 0.9;
                        let r = (255.0 * pulse) as u8;
                        let g = (100.0 * pulse) as u8;
                        let b = (80.0 * pulse) as u8;
                        canvas.set_colored(px, py, brightness, r, g, b);
                    }
                }
            }
        }
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
