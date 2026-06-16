use super::Animation;
use crate::render::Canvas;

/// Swing amplitude in radians (~54°).
const AMP: f64 = 0.95;
/// Full swing period (left-apex -> right-apex -> left-apex), seconds.
const PERIOD: f64 = 3.0;

/// Newton's cradle: a row of identical steel balls where energy transfers
/// cleanly from one side to the other on impact. Modeled with a single
/// continuous swing phase attributed to whichever side-group is currently
/// lifted, which reproduces the exact energy-conserving look with no
/// collision-resolution edge cases.
pub struct NewtonsCradle;

impl NewtonsCradle {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = (width, height, scale);
        NewtonsCradle
    }
}

impl Animation for NewtonsCradle {
    fn name(&self) -> &str {
        "newtons_cradle"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        canvas.clear();

        // Fewer balls on very narrow terminals.
        let n = if canvas.width >= 30 { 5 } else { 3 };
        let k = if n >= 5 { 2 } else { 1 };

        let spacing = (w * 0.9 / n as f64).min(h * 0.18).max(4.0);
        let radius = (spacing * 0.5 - 0.6).max(1.4);
        let pivot_y = h * 0.12;
        let base_y = h * 0.94;
        let rope = ((base_y - pivot_y) * 0.62).max(h * 0.3);
        let center = w * 0.5;
        let first = center - (n - 1) as f64 * 0.5 * spacing;

        let tau = std::f64::consts::TAU;
        // Continuous pendulum phase; positive swings the right group out,
        // negative swings the left group out.
        let swing = AMP * (tau / PERIOD * time).cos();

        // Draw the frame: top beam, two legs, base.
        let beam_left = first - spacing * 0.4;
        let beam_right = first + (n - 1) as f64 * spacing + spacing * 0.4;
        let frame: (u8, u8, u8) = (120, 120, 135);
        draw_line(canvas, beam_left, pivot_y, beam_right, pivot_y, 0.5, frame);
        draw_line(canvas, beam_left, pivot_y, beam_left, base_y, 0.4, frame);
        draw_line(canvas, beam_right, pivot_y, beam_right, base_y, 0.4, frame);
        draw_line(
            canvas,
            beam_left * 0.6 + beam_right * 0.4,
            base_y,
            beam_right,
            base_y,
            0.5,
            frame,
        );

        // Draw each ball: rope + shaded steel bob.
        for i in 0..n {
            let pivot_x = first + i as f64 * spacing;
            let is_left = i < k;
            let is_right = i >= n - k;
            let theta = if (is_right && swing > 0.0) || (is_left && swing < 0.0) {
                swing
            } else {
                0.0
            };
            let (st, ct) = theta.sin_cos();
            let bob_x = pivot_x + rope * st;
            let bob_y = pivot_y + rope * ct;

            draw_line(canvas, pivot_x, pivot_y, bob_x, bob_y, 0.3, (90, 90, 105));
            draw_ball(canvas, bob_x, bob_y, radius);
        }
    }
}

/// Draw a DDA-stepped line segment with the given brightness/color.
fn draw_line(
    canvas: &mut Canvas,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    brightness: f64,
    color: (u8, u8, u8),
) {
    let (r, g, b) = color;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    if steps == 0 {
        let ix = x0 as usize;
        let iy = y0 as usize;
        if ix < canvas.width && iy < canvas.height {
            canvas.set_colored(ix, iy, brightness, r, g, b);
        }
        return;
    }
    let sx = dx / steps as f64;
    let sy = dy / steps as f64;
    for s in 0..=steps {
        let x = x0 + sx * s as f64;
        let y = y0 + sy * s as f64;
        let ix = x as usize;
        let iy = y as usize;
        if ix < canvas.width && iy < canvas.height {
            canvas.set_colored(ix, iy, brightness, r, g, b);
        }
    }
}

/// Draw a shaded steel ball: light from the upper-left, shadow lower-right.
fn draw_ball(canvas: &mut Canvas, cx: f64, cy: f64, radius: f64) {
    let ri = radius.ceil() as i32;
    let icx = cx.round() as i32;
    let icy = cy.round() as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            if dist <= radius {
                let nx = dx as f64 / radius;
                let ny = dy as f64 / radius;
                // Light direction upper-left (-0.6, -0.6).
                let light = (-nx * 0.6 - ny * 0.6).clamp(0.0, 1.0);
                let shade = 0.15 + light * 0.85;
                let r = (55.0 + shade * 200.0) as u8;
                let g = (60.0 + shade * 200.0) as u8;
                let bl = (80.0 + shade * 180.0) as u8;
                let px = icx + dx;
                let py = icy + dy;
                if px >= 0 && py >= 0 {
                    let pxu = px as usize;
                    let pyu = py as usize;
                    if pxu < canvas.width && pyu < canvas.height {
                        canvas.set_colored(pxu, pyu, 0.5 + shade * 0.5, r, g, bl);
                    }
                }
            }
        }
    }
}
