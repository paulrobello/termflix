use super::Animation;
use crate::color::hsv_to_rgb;
use crate::render::Canvas;

const SIGMA: f64 = 10.0;
const RHO: f64 = 28.0;
const BETA: f64 = 8.0 / 3.0;
const TRAIL_MAX: usize = 1500;
const STEPS_PER_FRAME: usize = 16;
const ODE_DT: f64 = 0.005;

/// Strange attractor: a Lorenz system integrated with RK4 and rendered as a
/// slowly rotating, rainbow-fading trajectory drawn with soft glowing points.
pub struct StrangeAttractor {
    x: f64,
    y: f64,
    z: f64,
    trail: Vec<(f64, f64, f64)>,
}

impl StrangeAttractor {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = (width, height, scale);
        StrangeAttractor {
            x: 0.1,
            y: 0.0,
            z: 0.0,
            trail: Vec::with_capacity(TRAIL_MAX),
        }
    }

    /// Lorenz derivative at (x, y, z).
    fn deriv(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        (SIGMA * (y - x), x * (RHO - z) - y, x * y - BETA * z)
    }

    fn rk4_step(&mut self, dt: f64) {
        let (x, y, z) = (self.x, self.y, self.z);
        let (k1x, k1y, k1z) = Self::deriv(x, y, z);
        let (k2x, k2y, k2z) =
            Self::deriv(x + 0.5 * dt * k1x, y + 0.5 * dt * k1y, z + 0.5 * dt * k1z);
        let (k3x, k3y, k3z) =
            Self::deriv(x + 0.5 * dt * k2x, y + 0.5 * dt * k2y, z + 0.5 * dt * k2z);
        let (k4x, k4y, k4z) = Self::deriv(x + dt * k3x, y + dt * k3y, z + dt * k3z);
        self.x += dt / 6.0 * (k1x + 2.0 * k2x + 2.0 * k3x + k4x);
        self.y += dt / 6.0 * (k1y + 2.0 * k2y + 2.0 * k3y + k4y);
        self.z += dt / 6.0 * (k1z + 2.0 * k2z + 2.0 * k3z + k4z);
    }
}

impl Animation for StrangeAttractor {
    fn name(&self) -> &str {
        "strange_attractor"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        for _ in 0..STEPS_PER_FRAME {
            self.rk4_step(ODE_DT);
            self.trail.push((self.x, self.y, self.z));
            if self.trail.len() > TRAIL_MAX {
                self.trail.remove(0);
            }
        }

        canvas.clear();
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let s = w.min(h) / 38.0;
        let yaw = time * 0.10 + 0.6;
        let (sa, ca) = yaw.sin_cos();
        let n = self.trail.len().max(1);

        for (i, &(wx, wy, wz)) in self.trail.iter().enumerate() {
            // Center the attractor and treat z as vertical.
            let px = wx;
            let py = wz - 25.0;
            let pz = wy;
            // Rotate around the vertical (y) axis.
            let rx = px * ca - pz * sa;
            let rz = px * sa + pz * ca;
            let sx = cx + rx * s;
            let sy = cy - py * s;

            let depth = ((rz + 25.0) / 50.0).clamp(0.2, 1.0);
            let frac = i as f64 / n as f64;
            let hue = (frac * 0.7 + time * 0.03).rem_euclid(1.0);
            let (r, g, b) = hsv_to_rgb(hue, 0.95, 0.6 + 0.4 * frac);
            let bright = ((0.15 + 0.85 * frac) * depth).clamp(0.0, 1.0);
            plot_soft(canvas, sx, sy, bright, (r, g, b));
        }
    }
}

/// Plot a soft glowing point: a bright center with dimmer orthogonal neighbors.
fn plot_soft(canvas: &mut Canvas, x: f64, y: f64, bright: f64, color: (u8, u8, u8)) {
    let (r, g, b) = color;
    let ix = x.round() as i64;
    let iy = y.round() as i64;
    setpix(canvas, ix, iy, bright, r, g, b);
    let hb = bright * 0.45;
    setpix(canvas, ix + 1, iy, hb, r, g, b);
    setpix(canvas, ix - 1, iy, hb, r, g, b);
    setpix(canvas, ix, iy + 1, hb, r, g, b);
    setpix(canvas, ix, iy - 1, hb, r, g, b);
}

#[allow(clippy::too_many_arguments)]
fn setpix(canvas: &mut Canvas, x: i64, y: i64, bright: f64, r: u8, g: u8, b: u8) {
    if x >= 0 && y >= 0 {
        let xu = x as usize;
        let yu = y as usize;
        if xu < canvas.width && yu < canvas.height {
            canvas.set_colored(xu, yu, bright, r, g, b);
        }
    }
}
