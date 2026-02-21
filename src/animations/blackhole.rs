use super::Animation;
use crate::render::Canvas;

/// Accretion disk with gravitational lensing distortion, M87-inspired
pub struct Blackhole {
    _unused: (),
}

impl Blackhole {
    pub fn new() -> Self {
        Blackhole { _unused: () }
    }
}

impl Animation for Blackhole {
    fn name(&self) -> &str {
        "blackhole"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let bh_radius = (w.min(h) * 0.08).max(4.0);
        let disk_outer = bh_radius * 5.0;
        let disk_inner = bh_radius * 1.5;

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                let dx = fx - cx;
                let dy = fy - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                // Event horizon - pure black
                if dist < bh_radius {
                    continue;
                }

                // Photon ring - bright thin ring at ~1.5x event horizon
                let photon_ring_r = bh_radius * 1.5;
                let photon_dist = (dist - photon_ring_r).abs();
                if photon_dist < 1.0 {
                    let ring_bright = (1.0 - photon_dist) * 0.8;
                    canvas.set_colored(x, y, ring_bright, 255, 200, 100);
                    continue;
                }

                // Accretion disk
                if dist > disk_inner && dist < disk_outer {
                    // Disk is in the orbital plane - simulate viewing angle
                    // M87-style: asymmetric brightness due to relativistic beaming
                    let disk_frac = (dist - disk_inner) / (disk_outer - disk_inner);

                    // Doppler beaming - one side brighter than the other
                    let doppler = (angle - time * 0.3).cos() * 0.4 + 0.6;

                    // Spiral structure in the disk
                    let spiral =
                        ((angle * 3.0 - dist * 0.2 + time * 0.8).sin() * 0.3 + 0.7).max(0.0);

                    // Temperature gradient: hotter near center
                    let temperature = 1.0 - disk_frac * 0.7;

                    let intensity = temperature * doppler * spiral;

                    // Gravitational lensing: bend the disk appearance near the hole
                    let lensing = if dist < bh_radius * 3.0 {
                        let lens_factor = (dist - bh_radius) / (bh_radius * 2.0);
                        lens_factor.clamp(0.3, 1.0)
                    } else {
                        1.0
                    };

                    let v = (intensity * lensing).clamp(0.0, 1.0);

                    if v > 0.01 {
                        let (r, g, b) = accretion_color(v, temperature);
                        canvas.set_colored(x, y, v, r, g, b);
                    }
                } else if dist >= disk_outer {
                    // Faint glow beyond disk
                    let glow = ((dist - disk_outer) / (disk_outer * 0.5)).neg_exp_falloff();
                    if glow > 0.01 {
                        let r = (80.0 * glow) as u8;
                        let g = (40.0 * glow) as u8;
                        let b = (20.0 * glow) as u8;
                        canvas.set_colored(x, y, glow * 0.2, r, g, b);
                    }

                    // Background stars
                    let star_hash = ((fx * 127.1 + fy * 311.7).sin() * 43758.5453).fract().abs();
                    if star_hash > 0.998 {
                        // Gravitational lensing of stars near the black hole
                        let star_bright = if dist < disk_outer * 1.5 {
                            let stretch = (dist / (disk_outer * 1.5)).powi(2);
                            0.3 * stretch
                        } else {
                            0.3
                        };
                        let twinkle =
                            ((time * 2.0 + star_hash * 50.0).sin() * 0.5 + 0.5) * star_bright;
                        canvas.set_colored(x, y, twinkle, 200, 200, 230);
                    }
                }
            }
        }
    }
}

trait FalloffExt {
    fn neg_exp_falloff(self) -> f64;
}

impl FalloffExt for f64 {
    fn neg_exp_falloff(self) -> f64 {
        (-self * 3.0).exp()
    }
}

fn accretion_color(v: f64, temperature: f64) -> (u8, u8, u8) {
    // M87-inspired: orange-yellow core, reddish outer
    let t = temperature;
    if t > 0.7 {
        // Hot inner: bright orange-yellow
        let f = (t - 0.7) / 0.3;
        (255, (180.0 + 75.0 * f) as u8, (40.0 + 80.0 * f * v) as u8)
    } else if t > 0.4 {
        // Mid: orange-red
        let f = (t - 0.4) / 0.3;
        (
            (200.0 + 55.0 * f) as u8,
            (80.0 + 100.0 * f) as u8,
            (10.0 + 30.0 * f) as u8,
        )
    } else {
        // Cool outer: dark red
        let f = t / 0.4;
        (
            (80.0 + 120.0 * f) as u8,
            (20.0 + 60.0 * f) as u8,
            (5.0 + 5.0 * f) as u8,
        )
    }
}
