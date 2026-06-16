use super::Animation;
use crate::render::Canvas;
use noise::{NoiseFn, Perlin};
use rand::RngExt;

/// Heightmap side length (toroidal; the camera flies forever by wrapping).
const HM: usize = 256;
const NEAR: f64 = 1.0;
const FAR: f64 = 90.0;
const FOV: f64 = 0.55;

/// Raycaster terrain flyover: a voxel-style column renderer over a scrolling
/// ridged-noise heightmap, with elevation coloring and distance fog.
pub struct Terrain {
    heightmap: Vec<f64>,
    cam_z: f64,
}

impl Terrain {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = (width, height, scale);
        let noise = Perlin::new(rand::rng().random_range(0..u32::MAX));
        let mut heightmap = vec![0.0; HM * HM];
        for y in 0..HM {
            for x in 0..HM {
                heightmap[y * HM + x] = fbm_value(&noise, x as f64 * 0.045, y as f64 * 0.045);
            }
        }
        Terrain {
            heightmap,
            cam_z: 0.0,
        }
    }

    /// Toroidal heightmap sample in [0, 1].
    fn sample(&self, x: f64, z: f64) -> f64 {
        let xi = (x.round() as i64).rem_euclid(HM as i64) as usize;
        let zi = (z.round() as i64).rem_euclid(HM as i64) as usize;
        self.heightmap[zi * HM + xi]
    }
}

impl Animation for Terrain {
    fn name(&self) -> &str {
        "terrain"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        self.cam_z += dt * 8.0;
        let cam_x = time.sin() * 12.0;
        let cam_height = 0.40;
        let horizon = (h * 0.40) as i64;
        let horizon_us = (horizon.max(0) as usize).min(canvas.height);
        let focal = h * 0.9;

        canvas.clear();

        // Sky gradient above the horizon.
        for y in 0..=horizon_us {
            let tt = y as f64 / horizon_us.max(1) as f64;
            let r = (12.0 + tt * 55.0) as u8;
            let g = (16.0 + tt * 70.0) as u8;
            let b = (45.0 + tt * 120.0) as u8;
            for x in 0..canvas.width {
                canvas.set_colored(x, y, 0.5 + 0.4 * tt, r, g, b);
            }
        }

        // Terrain columns, far -> near so nearer ground overdraws the lower band.
        for px in 0..canvas.width {
            let rel = (px as f64 / (w - 1.0).max(1.0) - 0.5) * FOV;
            let (sa, ca) = rel.sin_cos();
            let mut z = FAR;
            while z > NEAR {
                let wx = cam_x + sa * z;
                let wz = self.cam_z + ca * z;
                let hh = self.sample(wx, wz);
                let proj = (cam_height - hh) * focal / z;
                let sy = (horizon as f64 + proj).round() as i64;
                let fog = (z / FAR).clamp(0.0, 1.0);
                let (cr, cg, cb) = terrain_color(hh, fog);
                let top = sy.max(0) as usize;
                if top < canvas.height {
                    for yy in top..canvas.height {
                        canvas.set_colored(px, yy, 1.0, cr, cg, cb);
                    }
                }
                z *= 0.90;
            }
        }
    }
}

/// Ridged fractal Brownian motion: sharp mountain ridges with valleys.
fn fbm_value(noise: &Perlin, x: f64, y: f64) -> f64 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for _ in 0..5 {
        let n = 1.0 - noise.get([x * freq, y * freq, 7.3]).abs();
        sum += amp * n * n;
        norm += amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    (sum / norm).clamp(0.0, 1.0)
}

/// Elevation-based terrain color blended toward haze with distance fog.
fn terrain_color(h: f64, fog: f64) -> (u8, u8, u8) {
    let (r, g, b) = if h < 0.22 {
        let t = h / 0.22;
        (15.0 + 25.0 * t, 35.0 + 55.0 * t, 80.0 + 70.0 * t)
    } else if h < 0.40 {
        let t = (h - 0.22) / 0.18;
        (110.0 + 50.0 * t, 100.0 + 60.0 * t, 70.0 + 20.0 * t)
    } else if h < 0.68 {
        let t = (h - 0.40) / 0.28;
        (90.0 + 50.0 * t, 120.0 - 10.0 * t, 60.0)
    } else if h < 0.85 {
        let t = (h - 0.68) / 0.17;
        (120.0 + 50.0 * t, 110.0 + 40.0 * t, 95.0 + 30.0 * t)
    } else {
        let t = ((h - 0.85) / 0.15).clamp(0.0, 1.0);
        (200.0 + 55.0 * t, 200.0 + 55.0 * t, 210.0 + 45.0 * t)
    };
    let f = fog * fog * 0.85;
    let r = r * (1.0 - f) + 120.0 * f;
    let g = g * (1.0 - f) + 130.0 * f;
    let b = b * (1.0 - f) + 160.0 * f;
    (r as u8, g as u8, b as u8)
}
