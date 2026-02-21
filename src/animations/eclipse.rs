use super::Animation;
use crate::render::Canvas;

/// Moon crossing sun with corona rays radiating outward
pub struct Eclipse {
    phase: f64,
}

impl Eclipse {
    pub fn new() -> Self {
        Eclipse { phase: 0.0 }
    }
}

impl Animation for Eclipse {
    fn name(&self) -> &str {
        "eclipse"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let sun_r = (w.min(h) * 0.18).max(8.0);
        let moon_r = sun_r * 0.95;

        // Moon moves across the sun in a slow cycle
        self.phase += dt * 0.15;
        let cycle = (self.phase * 0.5).sin();
        let moon_offset_x = cycle * sun_r * 2.5;
        let moon_offset_y = (self.phase * 0.3).sin() * sun_r * 0.3;
        let moon_cx = cx + moon_offset_x;
        let moon_cy = cy + moon_offset_y;

        // How much of the sun is covered
        let moon_dist = ((moon_cx - cx).powi(2) + (moon_cy - cy).powi(2)).sqrt();
        let coverage = (1.0 - moon_dist / (sun_r + moon_r)).clamp(0.0, 1.0);

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                let dx_sun = fx - cx;
                let dy_sun = fy - cy;
                let dist_sun = (dx_sun * dx_sun + dy_sun * dy_sun).sqrt();

                let dx_moon = fx - moon_cx;
                let dy_moon = fy - moon_cy;
                let dist_moon = (dx_moon * dx_moon + dy_moon * dy_moon).sqrt();

                let in_sun = dist_sun < sun_r;
                let in_moon = dist_moon < moon_r;

                if in_moon {
                    // Moon surface - dark with slight edge glow when eclipsing
                    if in_sun && coverage > 0.3 {
                        let edge = 1.0 - (moon_r - dist_moon) / (moon_r * 0.15);
                        if edge > 0.0 {
                            let glow = edge.clamp(0.0, 1.0) * coverage;
                            canvas.set_colored(x, y, glow * 0.3, 255, 200, 150);
                        } else {
                            canvas.set_colored(x, y, 0.05, 30, 30, 40);
                        }
                    } else {
                        canvas.set_colored(x, y, 0.05, 30, 30, 40);
                    }
                } else if in_sun {
                    // Sun surface
                    let edge = (sun_r - dist_sun) / sun_r;
                    let limb_darkening = edge.sqrt();
                    let r = (255.0 * limb_darkening) as u8;
                    let g = (220.0 * limb_darkening) as u8;
                    let b = (50.0 * limb_darkening) as u8;
                    canvas.set_colored(x, y, limb_darkening, r, g, b);
                } else {
                    // Corona and rays
                    let corona_dist = dist_sun - sun_r;
                    let max_corona = sun_r * 1.5;

                    if corona_dist < max_corona {
                        let angle = dy_sun.atan2(dx_sun);
                        let ray_count = 12.0;
                        let ray = ((angle * ray_count + time * 0.5).sin() * 0.5 + 0.5).powi(2);
                        let ray2 =
                            ((angle * ray_count * 0.5 - time * 0.3).sin() * 0.5 + 0.5).powi(3);

                        let falloff = (1.0 - corona_dist / max_corona).powi(2);
                        let corona_intensity =
                            falloff * (0.3 + ray * 0.5 + ray2 * 0.3) * coverage.max(0.2);

                        if corona_intensity > 0.01 {
                            let r = (255.0 * corona_intensity.min(1.0)) as u8;
                            let g = (200.0 * corona_intensity.min(1.0)) as u8;
                            let b = (100.0 * (corona_intensity * 0.5).min(1.0)) as u8;
                            canvas.set_colored(x, y, corona_intensity, r, g, b);
                        }
                    }

                    // Stars in background
                    let star_hash = ((fx * 127.1 + fy * 311.7).sin() * 43758.5453).fract().abs();
                    if star_hash > 0.997 {
                        let twinkle = ((time * 3.0 + star_hash * 100.0).sin() * 0.5 + 0.5) * 0.3;
                        canvas.set_colored(x, y, twinkle + 0.1, 200, 200, 220);
                    }
                }
            }
        }
    }
}
