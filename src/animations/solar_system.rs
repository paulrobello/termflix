use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Trail positions remembered per planet.
const TRAIL_LEN: usize = 24;
/// Number of asteroids in the belt.
const ASTEROID_COUNT: usize = 120;

/// Static planet definition: (orbit_frac, period, glow, r, g, b, moons, has_ring).
type PlanetDef = (f64, f64, usize, u8, u8, u8, u8, bool);

struct Planet {
    /// Orbit radius as a fraction of the max orbit radius.
    orbit_frac: f64,
    angle: f64,
    /// Seconds per full orbit.
    period: f64,
    /// Glow radius in pixels.
    glow: usize,
    r: u8,
    g: u8,
    b: u8,
    moons: u8,
    has_ring: bool,
    trail: Vec<(f64, f64)>,
}

struct Asteroid {
    orbit_frac: f64,
    angle: f64,
    period: f64,
}

/// Solar system with planets, moons, rings, and a drifting asteroid belt.
pub struct SolarSystem {
    planets: Vec<Planet>,
    asteroids: Vec<Asteroid>,
}

impl SolarSystem {
    #[allow(unused_variables)]
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let _ = (width, height, scale);
        let mut rng = rand::rng();

        let defs: [PlanetDef; 8] = [
            (0.085, 2.0, 1, 170, 170, 170, 0, false), // Mercury
            (0.13, 3.2, 1, 220, 190, 120, 0, false),  // Venus
            (0.18, 4.6, 1, 90, 150, 235, 1, false),   // Earth
            (0.24, 6.2, 1, 220, 95, 70, 2, false),    // Mars
            (0.36, 9.5, 2, 220, 180, 130, 4, false),  // Jupiter
            (0.47, 12.5, 2, 230, 210, 160, 2, true),  // Saturn
            (0.57, 16.0, 2, 150, 220, 235, 1, false), // Uranus
            (0.66, 20.0, 2, 90, 130, 235, 1, false),  // Neptune
        ];
        let planets = defs
            .iter()
            .map(|&(of, period, glow, r, g, b, moons, ring)| Planet {
                orbit_frac: of,
                angle: rng.random_range(0.0..std::f64::consts::TAU),
                period,
                glow,
                r,
                g,
                b,
                moons,
                has_ring: ring,
                trail: Vec::with_capacity(TRAIL_LEN),
            })
            .collect();

        // Asteroid belt between Mars (0.24) and Jupiter (0.36).
        let mut asteroids = Vec::with_capacity(ASTEROID_COUNT);
        for _ in 0..ASTEROID_COUNT {
            asteroids.push(Asteroid {
                orbit_frac: rng.random_range(0.285..0.335),
                angle: rng.random_range(0.0..std::f64::consts::TAU),
                period: rng.random_range(7.0..11.0),
            });
        }

        SolarSystem { planets, asteroids }
    }
}

impl Animation for SolarSystem {
    fn name(&self) -> &str {
        "solar_system"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        canvas.clear();

        let cx = w * 0.5;
        let cy = h * 0.5;
        let max_r = w.min(h) * 0.46;
        let tau = std::f64::consts::TAU;

        // Advance orbital angles.
        for p in &mut self.planets {
            p.angle = (p.angle + tau / p.period * dt).rem_euclid(tau);
            let r = p.orbit_frac * max_r;
            let px = cx + p.angle.cos() * r;
            let py = cy + p.angle.sin() * r;
            p.trail.push((px, py));
            if p.trail.len() > TRAIL_LEN {
                p.trail.remove(0);
            }
        }
        for a in &mut self.asteroids {
            a.angle = (a.angle + tau / a.period * dt).rem_euclid(tau);
        }

        // Faint orbit guide rings.
        for p in &self.planets {
            plot_circle(canvas, cx, cy, p.orbit_frac * max_r, 0.05, (55, 65, 90));
        }

        // Asteroid belt.
        for a in &self.asteroids {
            let r = a.orbit_frac * max_r;
            let px = cx + a.angle.cos() * r;
            let py = cy + a.angle.sin() * r;
            canvas.set_colored(px as usize, py as usize, 0.35, 140, 130, 110);
        }

        // Sun with corona.
        radial_glow(
            canvas,
            cx,
            cy,
            (max_r * 0.06).max(3.0),
            (255, 230, 140),
            0.9,
        );
        radial_glow(
            canvas,
            cx,
            cy,
            (max_r * 0.035).max(2.0),
            (255, 255, 220),
            1.0,
        );

        // Planets.
        for p in &self.planets {
            let r = p.orbit_frac * max_r;
            let px = cx + p.angle.cos() * r;
            let py = cy + p.angle.sin() * r;

            // Trail.
            let tl = p.trail.len();
            for (i, &(tx, ty)) in p.trail.iter().enumerate() {
                let t = (i + 1) as f64 / tl as f64;
                canvas.set_colored(tx as usize, ty as usize, t * 0.35, p.r, p.g, p.b);
            }

            // Saturn's ring (behind body).
            if p.has_ring {
                plot_circle(canvas, px, py, p.glow as f64 + 2.5, 0.4, (210, 190, 140));
                plot_circle(canvas, px, py, p.glow as f64 + 3.5, 0.25, (180, 160, 120));
            }

            // Glow + body.
            radial_glow(canvas, px, py, p.glow as f64, (p.r, p.g, p.b), 0.5);
            canvas.set_colored(px as usize, py as usize, 1.0, p.r, p.g, p.b);

            // Moons.
            for m in 0..p.moons {
                let ma = p.angle * 3.0 + m as f64 * (tau / p.moons as f64);
                let md = p.glow as f64 + 2.5;
                let mx = px + ma.cos() * md;
                let my = py + ma.sin() * md;
                canvas.set_colored(mx as usize, my as usize, 0.8, 205, 205, 210);
            }
        }
    }
}

/// Plot a faint circle outline by sampling points around its circumference.
fn plot_circle(
    canvas: &mut Canvas,
    cx: f64,
    cy: f64,
    radius: f64,
    brightness: f64,
    color: (u8, u8, u8),
) {
    let (r, g, b) = color;
    if radius < 0.5 {
        return;
    }
    let steps = (radius * 6.0).max(12.0) as usize;
    let tau = std::f64::consts::TAU;
    for i in 0..steps {
        let a = i as f64 / steps as f64 * tau;
        let x = cx + a.cos() * radius;
        let y = cy + a.sin() * radius;
        let ix = x as usize;
        let iy = y as usize;
        if ix < canvas.width && iy < canvas.height {
            canvas.set_colored(ix, iy, brightness, r, g, b);
        }
    }
}

/// Draw a soft radial glow centered at (cx, cy).
fn radial_glow(
    canvas: &mut Canvas,
    cx: f64,
    cy: f64,
    radius: f64,
    color: (u8, u8, u8),
    intensity: f64,
) {
    let (r, g, b) = color;
    let ri = radius.ceil() as i32;
    let icx = cx.round() as i32;
    let icy = cy.round() as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            if dist <= radius {
                let falloff = 1.0 - dist / radius;
                let bright = falloff * intensity;
                let px = icx + dx;
                let py = icy + dy;
                if px >= 0 && py >= 0 {
                    let pxu = px as usize;
                    let pyu = py as usize;
                    if pxu < canvas.width && pyu < canvas.height {
                        canvas.set_colored(pxu, pyu, bright, r, g, b);
                    }
                }
            }
        }
    }
}
