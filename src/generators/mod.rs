use rand::RngExt;

/// A single particle managed by the ParticleSystem.
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub max_life: f64,
}

impl Particle {
    /// Returns normalized age (0.0 = just born, 1.0 = about to die).
    #[inline]
    pub fn age(&self) -> f64 {
        1.0 - (self.life / self.max_life).clamp(0.0, 1.0)
    }

    /// Returns remaining life fraction (1.0 = full, 0.0 = dead).
    #[inline]
    pub fn life_frac(&self) -> f64 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

/// A color stop in a gradient.
#[derive(Clone, Copy)]
pub struct ColorStop {
    pub t: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Linear gradient between color stops. Samples by parameter t in 0.0..=1.0.
#[derive(Clone)]
pub struct ColorGradient {
    stops: Vec<ColorStop>,
}

impl ColorGradient {
    /// Create a gradient from a list of color stops. Stops should be sorted by t.
    pub fn new(stops: Vec<ColorStop>) -> Self {
        assert!(stops.len() >= 2, "ColorGradient requires at least 2 stops");
        ColorGradient { stops }
    }

    /// Sample the gradient at parameter t (0.0..=1.0).
    pub fn sample(&self, t: f64) -> (u8, u8, u8) {
        let t = t.clamp(0.0, 1.0);
        if t <= self.stops[0].t {
            return (self.stops[0].r, self.stops[0].g, self.stops[0].b);
        }
        let last = &self.stops[self.stops.len() - 1];
        if t >= last.t {
            return (last.r, last.g, last.b);
        }
        for i in 0..self.stops.len() - 1 {
            let a = &self.stops[i];
            let b = &self.stops[i + 1];
            if t >= a.t && t <= b.t {
                let frac = (t - a.t) / (b.t - a.t);
                let r = (a.r as f64 + (b.r as f64 - a.r as f64) * frac) as u8;
                let g = (a.g as f64 + (b.g as f64 - a.g as f64) * frac) as u8;
                let bl = (a.b as f64 + (b.b as f64 - a.b as f64) * frac) as u8;
                return (r, g, bl);
            }
        }
        (last.r, last.g, last.b)
    }
}

/// Configuration for a particle emitter.
#[derive(Clone)]
pub struct EmitterConfig {
    /// Emitter position (x, y).
    pub x: f64,
    pub y: f64,
    /// Spread angle in radians (0 = laser, TAU = omnidirectional).
    pub spread: f64,
    /// Base emission angle in radians (0 = right, PI/2 = down).
    pub angle: f64,
    /// Min/max initial speed.
    pub speed_min: f64,
    pub speed_max: f64,
    /// Min/max particle lifetime in seconds.
    pub life_min: f64,
    pub life_max: f64,
    /// Gravity applied each frame (positive = downward).
    pub gravity: f64,
    /// Drag multiplier per frame (0.99 = slight drag, 1.0 = none).
    pub drag: f64,
    /// Wind force (x component).
    pub wind: f64,
    /// Color gradient sampled by particle age.
    pub gradient: ColorGradient,
}

/// A reusable particle system with configurable emitter.
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub config: EmitterConfig,
    capacity: usize,
}

impl ParticleSystem {
    /// Create a new particle system with the given capacity.
    pub fn new(config: EmitterConfig, capacity: usize) -> Self {
        ParticleSystem {
            particles: Vec::with_capacity(capacity),
            config,
            capacity,
        }
    }

    /// Emit `count` particles from the emitter.
    pub fn emit(&mut self, count: usize) {
        let mut rng = rand::rng();
        for _ in 0..count {
            if self.particles.len() >= self.capacity {
                break;
            }
            let half_spread = self.config.spread * 0.5;
            let angle = self.config.angle + rng.random_range(-half_spread..=half_spread);
            let speed = rng.random_range(self.config.speed_min..=self.config.speed_max);
            let life = rng.random_range(self.config.life_min..=self.config.life_max);
            self.particles.push(Particle {
                x: self.config.x,
                y: self.config.y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                life,
                max_life: life,
            });
        }
    }

    /// Emit a single particle with explicit velocity (for custom spawning).
    pub fn emit_at(&mut self, x: f64, y: f64, vx: f64, vy: f64, life: f64) {
        if self.particles.len() >= self.capacity {
            return;
        }
        self.particles.push(Particle {
            x,
            y,
            vx,
            vy,
            life,
            max_life: life,
        });
    }

    /// Update all particles: apply physics, remove dead particles.
    pub fn update(&mut self, dt: f64) {
        for p in &mut self.particles {
            p.vx += self.config.wind * dt;
            p.vy += self.config.gravity * dt;
            p.vx *= self.config.drag;
            p.vy *= self.config.drag;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Draw all particles to the canvas using the gradient.
    pub fn draw(&self, canvas: &mut crate::render::Canvas) {
        for p in &self.particles {
            let ix = p.x as usize;
            let iy = p.y as usize;
            if ix < canvas.width && iy < canvas.height {
                let age = p.age();
                let (r, g, b) = self.config.gradient.sample(age);
                let brightness = p.life_frac();
                canvas.set_colored(ix, iy, brightness, r, g, b);
            }
        }
    }

    /// Number of active particles.
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Clear all particles.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.particles.clear();
    }
}
