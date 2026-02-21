use crate::render::Canvas;
use super::Animation;

/// Spiral vortex with colorful swirling arms
pub struct Vortex {
    time_offset: f64,
}

impl Vortex {
    pub fn new(_width: usize, _height: usize, _scale: f64) -> Self {
        Vortex { time_offset: 0.0 }
    }
}

impl Animation for Vortex {
    fn name(&self) -> &str { "vortex" }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.time_offset += dt;
        let t = time * 0.6;
        let cx = canvas.width as f64 / 2.0;
        let cy = canvas.height as f64 / 2.0;
        let max_r = cx.min(cy) * 1.5;

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let dx = x as f64 - cx;
                let dy = (y as f64 - cy) * 1.8; // aspect ratio correction
                let dist = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                if dist < 1.0 || dist > max_r { continue; }

                let norm_dist = dist / max_r;

                // Spiral arms — angle offset increases with distance (logarithmic spiral)
                let spiral_angle = angle - norm_dist * 8.0 + t * 3.0;
                // Single dominant arm with faint secondary
                let arm1 = ((spiral_angle * 1.0).sin() * 0.5 + 0.5).powf(1.2);
                let arms = arm1;

                // Intensity: bright center, fading outward with spiral pattern
                let radial_fade = (1.0 - norm_dist).powf(0.6);
                let intensity = arms * radial_fade;

                if intensity < 0.05 { continue; }

                // Color based on angle + distance — creates rainbow spiral
                let hue = (angle / std::f64::consts::TAU + norm_dist * 0.5 + t * 0.15).fract();
                let (r, g, b) = hsv_to_rgb(hue, 0.7 + 0.3 * (1.0 - norm_dist), intensity);

                canvas.set_colored(x, y, intensity, r, g, b);
            }
        }

        // Bright core
        let core_r = (max_r * 0.08).max(3.0);
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let dx = x as f64 - cx;
                let dy = (y as f64 - cy) * 1.8;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < core_r {
                    let core_i = (1.0 - dist / core_r).powf(0.5);
                    let pulse = 0.8 + 0.2 * (t * 4.0).sin();
                    let brightness = core_i * pulse;
                    canvas.set_colored(x, y, brightness, 255, 240, (200.0 + 55.0 * core_i) as u8);
                }
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((h % 1.0) + 1.0) % 1.0;
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
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}
