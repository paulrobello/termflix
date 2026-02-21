use super::Animation;
use crate::render::Canvas;

/// Rotating DNA double helix
pub struct Dna;

impl Dna {
    pub fn new() -> Self {
        Dna
    }
}

impl Animation for Dna {
    fn name(&self) -> &str {
        "dna"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Ascii
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w / 2.0;
        let t = time * 2.0;

        canvas.clear();

        let amplitude = w * 0.2;
        let rungs_per_screen = 12.0;

        for y in 0..canvas.height {
            let fy = y as f64 / h;
            let phase = fy * rungs_per_screen * std::f64::consts::TAU / h * h + t;

            // Two strands
            let x1 = cx + (phase).sin() * amplitude;
            let x2 = cx + (phase + std::f64::consts::PI).sin() * amplitude;

            // Z-depth for visual ordering (determines which strand is in front)
            let z1 = phase.cos();
            let z2 = (phase + std::f64::consts::PI).cos();

            // Draw rungs (connecting lines) between strands when they're roughly at same depth
            let rung_interval = h / rungs_per_screen;
            let rung_phase = (y as f64 % rung_interval) / rung_interval;
            let near_rung = !(0.15..=0.85).contains(&rung_phase);

            if near_rung {
                let left = x1.min(x2) as usize;
                let right = x1.max(x2) as usize;
                for x in left..=right.min(canvas.width.saturating_sub(1)) {
                    if x < canvas.width {
                        let t_pos = (x as f64 - x1) / (x2 - x1 + 0.001);
                        // Base pair colors (A-T: red-green, C-G: blue-yellow)
                        let (r, g, b) = if t_pos < 0.5 {
                            (200, (100.0 + 100.0 * t_pos) as u8, 80)
                        } else {
                            (80, (200.0 - 100.0 * (t_pos - 0.5)) as u8, 200)
                        };
                        canvas.set_colored(x, y, 0.5, r, g, b);
                    }
                }
            }

            // Draw strand 1 and 2 (with depth-based brightness)
            let draw_strand = |canvas: &mut Canvas, sx: f64, z: f64, is_first: bool| {
                let brightness = (0.5 + z * 0.5).clamp(0.3, 1.0);
                let size = if z > 0.0 { 2 } else { 1 };
                let (r, g, b) = if is_first {
                    (
                        (80.0 + 175.0 * brightness) as u8,
                        (40.0 + 60.0 * brightness) as u8,
                        (180.0 + 75.0 * brightness) as u8,
                    )
                } else {
                    (
                        (180.0 + 75.0 * brightness) as u8,
                        (80.0 + 100.0 * brightness) as u8,
                        (40.0 + 60.0 * brightness) as u8,
                    )
                };
                for dx in 0..size {
                    let px = (sx as isize + dx as isize) as usize;
                    if px < canvas.width {
                        canvas.set_colored(px, y, brightness, r, g, b);
                    }
                }
            };

            // Draw back strand first, then front
            if z1 < z2 {
                draw_strand(canvas, x1, z1, true);
                draw_strand(canvas, x2, z2, false);
            } else {
                draw_strand(canvas, x2, z2, false);
                draw_strand(canvas, x1, z1, true);
            }
        }
    }
}
