use super::Animation;
use crate::render::Canvas;

/// Pendulum wave: a row of pendulums with slightly different periods
/// creating mesmerizing wave patterns as they go in and out of phase.
pub struct Pendulum;

impl Pendulum {
    #[allow(unused_variables)]
    pub fn new(_width: usize, _height: usize, _scale: f64) -> Self {
        Pendulum
    }
}

impl Animation for Pendulum {
    fn name(&self) -> &str {
        "pendulum"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;

        canvas.clear();

        let num_pendulums: usize = 20;
        let amplitude = 0.7; // fraction of available swing width
        let base_period = 3.0; // seconds for the slowest pendulum
        let delta_period = 0.15; // period increment per pendulum

        // Pivot points along the top, evenly spaced with margins
        let margin = w * 0.05;
        let spacing = (w - 2.0 * margin) / (num_pendulums - 1).max(1) as f64;

        // Rod length scales with canvas height
        let rod_length = h * 0.75;

        // Horizontal swing amplitude per pendulum
        let swing_amplitude = spacing * 0.45 * amplitude;

        // Trail settings: we store recent bob positions for a glow effect
        // by drawing dim copies at slightly earlier time offsets
        let trail_count = 4;

        for i in 0..num_pendulums {
            let pivot_x = margin + i as f64 * spacing;
            let pivot_y = h * 0.06;

            // Each pendulum has a slightly different period
            let period = base_period + i as f64 * delta_period;

            // Angle at current time
            let theta = amplitude * (std::f64::consts::TAU * time / period).sin();

            // Bob position
            let bob_x = pivot_x + theta * swing_amplitude;
            let bob_y = pivot_y + rod_length;

            // HSV color based on index
            let hue = i as f64 / num_pendulums as f64;
            let (r, g, b) = hsv_to_rgb(hue, 0.85, 1.0);
            let (r_dim, g_dim, b_dim) = hsv_to_rgb(hue, 0.6, 0.7);

            // Draw pivot point (small bright dot)
            let px = pivot_x as usize;
            let py = pivot_y as usize;
            if px < canvas.width && py < canvas.height {
                canvas.set_colored(px, py, 0.8, 220, 220, 220);
            }

            // Draw rod (line from pivot to bob) using DDA stepping
            let dx = bob_x - pivot_x;
            let dy = bob_y - pivot_y;
            let steps = (dx.abs().max(dy.abs())).ceil() as usize;
            if steps > 0 {
                let step_x = dx / steps as f64;
                let step_y = dy / steps as f64;
                for s in 1..steps {
                    let rx = pivot_x + step_x * s as f64;
                    let ry = pivot_y + step_y * s as f64;
                    let ix = rx as usize;
                    let iy = ry as usize;
                    if ix < canvas.width && iy < canvas.height {
                        // Fade rod from pivot to bob
                        let t_frac = s as f64 / steps as f64;
                        let rod_brightness = 0.15 + t_frac * 0.15;
                        canvas.set_colored(ix, iy, rod_brightness, r_dim, g_dim, b_dim);
                    }
                }
            }

            // Draw trail (ghost positions at slightly earlier times)
            for tr in (1..=trail_count).rev() {
                let trail_dt = tr as f64 * 0.04;
                let trail_theta =
                    amplitude * (std::f64::consts::TAU * (time - trail_dt) / period).sin();
                let trail_x = pivot_x + trail_theta * swing_amplitude;
                let trail_y = bob_y;
                let tx = trail_x as usize;
                let ty = trail_y as usize;
                if tx < canvas.width && ty < canvas.height {
                    let fade = 0.3 * (1.0 - tr as f64 / (trail_count + 1) as f64);
                    canvas.set_colored(tx, ty, fade, r_dim, g_dim, b_dim);
                }
            }

            // Draw bob with glow
            let glow_radius = 2.5;
            for dy in -(glow_radius as i32)..=(glow_radius as i32) {
                for dx in -(glow_radius as i32)..=(glow_radius as i32) {
                    let dist = ((dx * dx + dy * dy) as f64).sqrt();
                    if dist <= glow_radius {
                        let bx = (bob_x + dx as f64) as usize;
                        let by = (bob_y + dy as f64) as usize;
                        if bx < canvas.width && by < canvas.height {
                            let brightness = (1.0 - dist / glow_radius).powi(2);
                            canvas.set_colored(bx, by, brightness, r, g, b);
                        }
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
