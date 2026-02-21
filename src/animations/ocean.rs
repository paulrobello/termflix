use crate::render::Canvas;
use super::Animation;

/// Ocean waves with depth and foam
pub struct Ocean;

impl Ocean {
    pub fn new() -> Self { Ocean }
}

impl Animation for Ocean {
    fn name(&self) -> &str { "ocean" }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let t = time;

        canvas.clear();

        for x in 0..canvas.width {
            let fx = x as f64 / w;

            // Multiple wave layers with different frequencies and amplitudes
            let wave1 = (fx * 6.0 + t * 1.5).sin() * h * 0.08;
            let wave2 = (fx * 12.0 + t * 2.5).sin() * h * 0.04;
            let wave3 = (fx * 3.0 + t * 0.8).sin() * h * 0.12;
            let wave4 = (fx * 18.0 + t * 3.5).sin() * h * 0.02;

            let wave_height = wave1 + wave2 + wave3 + wave4;
            let surface = h * 0.35 + wave_height;

            for y in 0..canvas.height {
                let fy = y as f64;
                let depth = fy - surface;

                if depth < -2.0 {
                    // Sky
                    continue;
                } else if depth < 0.0 {
                    // Foam/crest
                    let foam = ((fx * 20.0 + t * 3.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                    let brightness = (0.7 + foam * 0.3).clamp(0.0, 1.0);
                    canvas.set_colored(x, y, brightness, 220, 240, 255);
                } else {
                    // Water body — darker with depth
                    let depth_ratio = (depth / (h * 0.65)).clamp(0.0, 1.0);
                    let brightness = (0.6 - depth_ratio * 0.45).clamp(0.1, 0.7);

                    // Underwater wave distortion
                    let underwater_wave = (fx * 8.0 + fy * 0.05 + t * 1.2).sin() * 0.1;
                    let b_mod = (brightness + underwater_wave).clamp(0.1, 0.7);

                    let r = (20.0 + 30.0 * (1.0 - depth_ratio)) as u8;
                    let g = (80.0 + 80.0 * (1.0 - depth_ratio) + underwater_wave * 30.0) as u8;
                    let b = (150.0 + 105.0 * (1.0 - depth_ratio * 0.5)) as u8;
                    canvas.set_colored(x, y, b_mod, r, g, b);
                }
            }
        }
    }
}
