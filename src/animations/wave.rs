use super::Animation;
use crate::render::Canvas;

/// Sine wave interference pattern
pub struct Wave {
    amplitude: f64,
    frequency: f64,
}

impl Wave {
    pub fn new() -> Self {
        Wave {
            amplitude: 0.5,
            frequency: 0.3,
        }
    }
}

impl Animation for Wave {
    fn name(&self) -> &str {
        "wave"
    }

    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(intensity) = params.intensity {
            self.amplitude = intensity.clamp(0.1, 1.0);
        }
        if let Some(cs) = params.color_shift {
            self.frequency = cs.clamp(0.05, 0.8);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("intensity", 0.1, 1.0), ("color_shift", 0.05, 0.8)]
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let t = time;

        // Two wave sources
        let s1x = w * 0.3 + (t * 0.5).cos() * w * 0.2;
        let s1y = h * 0.5 + (t * 0.7).sin() * h * 0.3;
        let s2x = w * 0.7 + (t * 0.3).sin() * w * 0.2;
        let s2y = h * 0.5 + (t * 0.4).cos() * h * 0.3;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                let d1 = ((fx - s1x).powi(2) + (fy - s1y).powi(2)).sqrt();
                let d2 = ((fx - s2x).powi(2) + (fy - s2y).powi(2)).sqrt();

                let wave1 = (d1 * self.frequency - t * 4.0).sin();
                let wave2 = (d2 * self.frequency - t * 3.5).sin();
                let combined = (wave1 + wave2) * self.amplitude;

                let v = (combined + 1.0) * 0.5; // normalize to 0..1

                let r = ((v * std::f64::consts::PI).sin() * 100.0 + 50.0) as u8;
                let g = ((v * std::f64::consts::PI * 0.7).sin() * 150.0 + 100.0) as u8;
                let b = ((v * std::f64::consts::PI * 1.3 + 1.0).sin() * 127.0 + 128.0) as u8;

                canvas.set_colored(x, y, v, r, g, b);
            }
        }
    }
}
