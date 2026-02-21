use super::Animation;
use crate::render::Canvas;

/// Rotating spiral pattern
pub struct Spiral;

impl Spiral {
    pub fn new() -> Self {
        Spiral
    }
}

impl Animation for Spiral {
    fn name(&self) -> &str {
        "spiral"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let max_r = (cx * cx + cy * cy).sqrt();
        let t = time * 1.5;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let r = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                // Spiral formula: brightness based on angle + radius offset
                let arms = 4.0;
                let spiral = (angle * arms + r * 0.15 - t * 3.0).sin();
                let fade = 1.0 - (r / max_r).clamp(0.0, 1.0);
                let v = ((spiral + 1.0) * 0.5 * fade).clamp(0.0, 1.0);

                if v > 0.05 {
                    let hue = (angle / std::f64::consts::TAU + 0.5 + t * 0.1).fract();
                    let (cr, cg, cb) = hsv_to_rgb(hue, 0.8, v);
                    canvas.set_colored(x, y, v, cr, cg, cb);
                } else {
                    canvas.set_colored(x, y, 0.0, 0, 0, 0);
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
