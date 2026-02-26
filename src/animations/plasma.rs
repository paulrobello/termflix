use super::Animation;
use crate::render::Canvas;

/// Classic plasma effect using overlapping sine waves
pub struct Plasma {
    /// Hue bias from external color_shift param: rotates the color palette independently
    hue_bias: f64,
}

impl Plasma {
    pub fn new() -> Self {
        Plasma { hue_bias: 0.0 }
    }
}

impl Animation for Plasma {
    fn name(&self) -> &str {
        "plasma"
    }

    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(cs) = params.color_shift {
            self.hue_bias = cs.clamp(0.0, 1.0);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("color_shift", 0.0, 1.0)]
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let t = time * 0.8;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64 / w * 8.0;
                let fy = y as f64 / h * 8.0;

                let v1 = (fx + t).sin();
                let v2 = ((fy * 1.5 + t * 0.7).sin() + (fx * 0.7 + t * 1.3).cos()) * 0.5;
                let v3 = ((fx * fx + fy * fy).sqrt() * 0.3 - t).sin();
                let v4 = ((fx * 0.5 + fy * 0.5 + t * 0.5).sin()) * 0.7;

                let v = (v1 + v2 + v3 + v4) * 0.25 + 0.5; // normalize to ~0..1
                let v = v.clamp(0.0, 1.0);

                let (r, g, b) = plasma_color(v, t, self.hue_bias);
                canvas.set_colored(x, y, v * 0.8 + 0.2, r, g, b);
            }
        }
    }
}

fn plasma_color(v: f64, t: f64, hue_bias: f64) -> (u8, u8, u8) {
    let bias = hue_bias * std::f64::consts::TAU;
    let r = ((v * std::f64::consts::PI + t * 0.3 + bias).sin() * 127.0 + 128.0) as u8;
    let g = ((v * std::f64::consts::PI * 1.5 + t * 0.5 + 2.0 + bias).sin() * 127.0 + 128.0) as u8;
    let b = ((v * std::f64::consts::PI * 2.0 + t * 0.7 + 4.0 + bias).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}
