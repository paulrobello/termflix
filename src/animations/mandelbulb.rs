use super::Animation;
use crate::render::Canvas;

const POWER: f64 = 8.0;
const MAX_ITER: usize = 7;
const BAILOUT: f64 = 2.0;
const MAX_STEPS: usize = 48;
const EPS: f64 = 0.0012;
const STEP: usize = 2;

type V3 = (f64, f64, f64);

/// 3D Mandelbulb: per-pixel sphere tracing of the Mandelbulb distance
/// estimator with normal-based diffuse shading, slow camera orbit, and a
/// cosine-palette color cycle.
pub struct Mandelbulb;

impl Mandelbulb {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = (width, height, scale);
        Mandelbulb
    }
}

impl Animation for Mandelbulb {
    fn name(&self) -> &str {
        "mandelbulb"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        canvas.clear();

        let angle = time * 0.12;
        let cam: V3 = (
            angle.sin() * 2.8,
            0.7 + (time * 0.2).sin() * 0.2,
            angle.cos() * 2.8,
        );
        let forward = vnorm(vsub((0.0, 0.0, 0.0), cam));
        let right = vnorm(vcross(forward, (0.0, 1.0, 0.0)));
        let up = vcross(right, forward);
        let light = vnorm((0.6, 0.9, 0.4));
        let aspect = w / h;
        let focal = 1.6;
        let hmax = (h - 1.0).max(1.0);
        let wmax = (w - 1.0).max(1.0);

        let mut py = 0;
        while py < canvas.height {
            let mut px = 0;
            while px < canvas.width {
                let ndc_x = (px as f64 / wmax - 0.5) * 2.0 * aspect;
                let ndc_y = (0.5 - py as f64 / hmax) * 2.0;
                let dir = vnorm(vadd(
                    vadd(vscale(forward, focal), vscale(right, ndc_x)),
                    vscale(up, ndc_y),
                ));
                let (hit, dist, orb) = raymarch(cam, dir);

                let (cr, cg, cb, br) = if hit {
                    let p = vadd(cam, vscale(dir, dist));
                    let n = vnorm(normal_at(p));
                    let diff = vdot(n, light).max(0.0);
                    let ao = (1.0 - orb as f64 / MAX_ITER as f64 * 0.6).max(0.0);
                    let bright = (0.18 + 0.82 * diff * ao).clamp(0.0, 1.0);
                    let t = orb as f64 / MAX_ITER as f64 + time * 0.08;
                    let (pr, pg, pb) = palette(t);
                    (
                        (pr * 255.0) as u8,
                        (pg * 255.0) as u8,
                        (pb * 255.0) as u8,
                        bright,
                    )
                } else {
                    (6, 8, 20, 0.05)
                };

                for dyy in 0..STEP {
                    for dxx in 0..STEP {
                        let xx = px + dxx;
                        let yy = py + dyy;
                        if xx < canvas.width && yy < canvas.height {
                            canvas.set_colored(xx, yy, br, cr, cg, cb);
                        }
                    }
                }
                px += STEP;
            }
            py += STEP;
        }
    }
}

/// Mandelbulb distance estimator; returns (distance, iteration count).
fn bulb_de(p: V3) -> (f64, usize) {
    let mut z = p;
    let mut dr = 1.0;
    let mut r = 0.0;
    let mut i = 0;
    while i < MAX_ITER {
        r = vlen(z);
        if r > BAILOUT {
            break;
        }
        let r_safe = r.max(1e-12);
        let theta = (z.2 / r_safe).clamp(-1.0, 1.0).acos();
        let phi = z.1.atan2(z.0);
        dr = r_safe.powf(POWER - 1.0) * POWER * dr + 1.0;
        let zr = r_safe.powf(POWER);
        let nt = theta * POWER;
        let np = phi * POWER;
        let st = nt.sin();
        let ct = nt.cos();
        let sp = np.sin();
        let cp = np.cos();
        z = (zr * st * cp + p.0, zr * st * sp + p.1, zr * ct + p.2);
        i += 1;
    }
    let r_safe = r.max(1e-12);
    (0.5 * r_safe.ln() * r_safe / dr, i)
}

fn raymarch(ro: V3, rd: V3) -> (bool, f64, usize) {
    let mut t = 0.02;
    let mut steps = 0;
    while t < 6.0 && steps < MAX_STEPS {
        let p = vadd(ro, vscale(rd, t));
        let (d, orb) = bulb_de(p);
        if d < EPS {
            return (true, t, orb);
        }
        t += d.max(EPS);
        steps += 1;
    }
    (false, t, 0)
}

fn normal_at(p: V3) -> V3 {
    let e = 0.0015;
    let dx = bulb_de((p.0 + e, p.1, p.2)).0 - bulb_de((p.0 - e, p.1, p.2)).0;
    let dy = bulb_de((p.0, p.1 + e, p.2)).0 - bulb_de((p.0, p.1 - e, p.2)).0;
    let dz = bulb_de((p.0, p.1, p.2 + e)).0 - bulb_de((p.0, p.1, p.2 - e)).0;
    vnorm((dx, dy, dz))
}

/// Inigo Quilez cosine palette, returns RGB in [0, 1].
fn palette(t: f64) -> (f64, f64, f64) {
    let tau = std::f64::consts::TAU;
    (
        0.5 + 0.5 * (tau * (t + 0.0)).cos(),
        0.5 + 0.5 * (tau * (t + 0.10)).cos(),
        0.5 + 0.5 * (tau * (t + 0.20)).cos(),
    )
}

fn vadd(a: V3, b: V3) -> V3 {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2)
}
fn vsub(a: V3, b: V3) -> V3 {
    (a.0 - b.0, a.1 - b.1, a.2 - b.2)
}
fn vscale(a: V3, s: f64) -> V3 {
    (a.0 * s, a.1 * s, a.2 * s)
}
fn vdot(a: V3, b: V3) -> f64 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}
fn vcross(a: V3, b: V3) -> V3 {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}
fn vlen(a: V3) -> f64 {
    vdot(a, a).sqrt()
}
fn vnorm(a: V3) -> V3 {
    let l = vlen(a).max(1e-12);
    (a.0 / l, a.1 / l, a.2 / l)
}
