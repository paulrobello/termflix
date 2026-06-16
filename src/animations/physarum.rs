use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Sensor aperture offset (radians) for left/right sensing relative to heading.
const SENSOR_ANGLE: f64 = 0.5;
/// Heading change applied when steering toward a stronger trail sample.
const ROTATION_ANGLE: f64 = 0.35;
/// Distance ahead at which an agent samples the trail grid.
const SENSOR_DIST: f64 = 6.0;
/// Forward step size per simulation tick (pixels).
const STEP_SIZE: f64 = 1.0;
/// Trail value deposited at an agent's current cell each tick.
const DEPOSIT: f64 = 5.0;
/// Fraction of trail removed each tick (diffusion + decay combined).
const DECAY: f64 = 0.05;
/// Fixed simulation timestep (seconds), decoupled from render frame rate.
const STEP_INTERVAL: f64 = 1.0 / 30.0;
/// Max simulation steps per frame to avoid a spiral-of-death on slow terminals.
const MAX_STEPS_PER_FRAME: usize = 4;

struct Agent {
    x: f64,
    y: f64,
    angle: f64,
}

/// Physarum (slime mold) agent simulation: thousands of agents deposit and
/// follow a shared pheromone trail, self-organizing into organic networks.
pub struct Physarum {
    width: usize,
    height: usize,
    scale: f64,
    grid: Vec<f64>,
    grid_back: Vec<f64>,
    agents: Vec<Agent>,
    step_timer: f64,
}

impl Physarum {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut p = Physarum {
            width: 1,
            height: 1,
            scale,
            grid: Vec::new(),
            grid_back: Vec::new(),
            agents: Vec::new(),
            step_timer: 0.0,
        };
        p.init(width.max(1), height.max(1));
        p
    }

    /// (Re)allocate the trail grid and spawn a fresh agent population.
    fn init(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        let n = width * height;
        self.grid = vec![0.0; n];
        self.grid_back = vec![0.0; n];

        let count = ((n as f64 / 30.0) * self.scale.max(0.25)).clamp(400.0, 5000.0) as usize;
        let mut rng = rand::rng();
        self.agents.clear();
        self.agents.reserve(count);
        let cx = width as f64 * 0.5;
        let cy = height as f64 * 0.5;
        let radius = width.min(height) as f64 * 0.25;
        for _ in 0..count {
            let a = rng.random_range(0.0..std::f64::consts::TAU);
            let r = rng.random_range(0.0..radius);
            self.agents.push(Agent {
                x: cx + a.cos() * r,
                y: cy + a.sin() * r,
                angle: rng.random_range(0.0..std::f64::consts::TAU),
            });
        }
    }

    /// Sample the trail grid at float coordinates with toroidal wrapping.
    fn sample(grid: &[f64], width: usize, height: usize, x: f64, y: f64) -> f64 {
        let xi = x.round() as i64;
        let yi = y.round() as i64;
        let xi = xi.rem_euclid(width as i64) as usize;
        let yi = yi.rem_euclid(height as i64) as usize;
        grid[yi * width + xi]
    }

    /// One simulation tick: sense, steer, move, deposit, then diffuse + decay.
    fn step(&mut self) {
        let width = self.width;
        let height = self.height;
        let wf = width as f64;
        let hf = height as f64;
        let mut rng = rand::rng();

        // Sense + steer + move (read grid field, mutate agents field — disjoint).
        for a in &mut self.agents {
            let (ca, sa) = a.angle.sin_cos();
            let (lca, lsa) = (a.angle - SENSOR_ANGLE).sin_cos();
            let (rca, rsa) = (a.angle + SENSOR_ANGLE).sin_cos();

            let val_l = Self::sample(
                &self.grid,
                width,
                height,
                a.x + lca * SENSOR_DIST,
                a.y + lsa * SENSOR_DIST,
            );
            let val_c = Self::sample(
                &self.grid,
                width,
                height,
                a.x + ca * SENSOR_DIST,
                a.y + sa * SENSOR_DIST,
            );
            let val_r = Self::sample(
                &self.grid,
                width,
                height,
                a.x + rca * SENSOR_DIST,
                a.y + rsa * SENSOR_DIST,
            );

            if val_l > val_r {
                a.angle -= ROTATION_ANGLE;
            } else if val_r > val_l {
                a.angle += ROTATION_ANGLE;
            } else if val_c < val_l {
                // Off-center and weaker ahead: randomize to explore.
                a.angle += if rng.random_range(0.0..1.0) < 0.5 {
                    ROTATION_ANGLE
                } else {
                    -ROTATION_ANGLE
                };
            }

            a.x = (a.x + a.angle.cos() * STEP_SIZE).rem_euclid(wf);
            a.y = (a.y + a.angle.sin() * STEP_SIZE).rem_euclid(hf);
        }

        // Deposit at new positions (read agents field, mutate grid field — disjoint).
        for a in &self.agents {
            let xi = a.x as usize;
            let yi = a.y as usize;
            self.grid[yi * width + xi] += DEPOSIT;
        }

        // Diffuse (3x3 box average) + decay into the back buffer, then swap.
        for y in 0..height {
            for x in 0..width {
                let mut sum = 0.0;
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let nx = (x as i64 + dx).rem_euclid(width as i64) as usize;
                        let ny = (y as i64 + dy).rem_euclid(height as i64) as usize;
                        sum += self.grid[ny * width + nx];
                    }
                }
                self.grid_back[y * width + x] = (sum / 9.0) * (1.0 - DECAY);
            }
        }
        std::mem::swap(&mut self.grid, &mut self.grid_back);
    }
}

impl Animation for Physarum {
    fn name(&self) -> &str {
        "physarum"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Rebuild grid if the canvas changed size.
        if canvas.width != self.width || canvas.height != self.height {
            self.init(canvas.width, canvas.height);
        }

        self.step_timer += dt;
        let mut steps = 0;
        while self.step_timer >= STEP_INTERVAL && steps < MAX_STEPS_PER_FRAME {
            self.step();
            self.step_timer -= STEP_INTERVAL;
            steps += 1;
        }
        if self.step_timer > STEP_INTERVAL * MAX_STEPS_PER_FRAME as f64 {
            self.step_timer = 0.0;
        }

        // Render trail grid to canvas with an intensity heatmap.
        canvas.clear();
        for y in 0..self.height {
            for x in 0..self.width {
                let v = self.grid[y * self.width + x];
                if v > 0.03 {
                    let intensity = (v / 4.0).clamp(0.0, 1.0);
                    let hue = 0.66 * (1.0 - intensity);
                    let (r, g, b) = hsv_to_rgb(hue, 0.9, 0.4 + 0.6 * intensity);
                    canvas.set_colored(x, y, intensity, r, g, b);
                }
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = h.rem_euclid(1.0);
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
