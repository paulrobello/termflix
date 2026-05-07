use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// A single cellular automaton ruleset defined in B/S notation.
struct Ruleset {
    #[allow(dead_code)]
    name: &'static str,
    #[allow(dead_code)]
    notation: &'static str,
    /// Birth rules: neighbor counts that birth a dead cell
    birth: [bool; 9],
    /// Survival rules: neighbor counts that keep a live cell alive
    survival: [bool; 9],
}

impl Ruleset {
    const fn new(
        name: &'static str,
        notation: &'static str,
        birth_counts: &[u8],
        survival_counts: &[u8],
    ) -> Self {
        let mut birth = [false; 9];
        let mut survival = [false; 9];
        let mut i = 0;
        while i < birth_counts.len() {
            birth[birth_counts[i] as usize] = true;
            i += 1;
        }
        let mut i = 0;
        while i < survival_counts.len() {
            survival[survival_counts[i] as usize] = true;
            i += 1;
        }
        Ruleset {
            name,
            notation,
            birth,
            survival,
        }
    }

    fn next_state(&self, alive: bool, neighbors: u8) -> bool {
        let n = neighbors as usize;
        if alive {
            self.survival[n]
        } else {
            self.birth[n]
        }
    }
}

const RULESETS: &[Ruleset] = &[
    Ruleset::new("Conway's Life", "B3/S23", &[3], &[2, 3]),
    Ruleset::new("Highlife", "B36/S23", &[3, 6], &[2, 3]),
    Ruleset::new(
        "Day & Night",
        "B3678/S34678",
        &[3, 6, 7, 8],
        &[3, 4, 6, 7, 8],
    ),
    Ruleset::new("Seeds", "B2/S", &[2], &[]),
    Ruleset::new("Diamoeba", "B35678/S5678", &[3, 5, 6, 7, 8], &[5, 6, 7, 8]),
    Ruleset::new("Replicator", "B1357/S1357", &[1, 3, 5, 7], &[1, 3, 5, 7]),
];

/// Maximum cell age before color stops shifting (caps gradient lookup).
const MAX_AGE: u16 = 120;

/// Cellular automata animation cycling through multiple rulesets.
pub struct Automata {
    width: usize,
    height: usize,
    /// Current cell states (double-buffered via swap)
    grid: Vec<bool>,
    /// Next generation buffer
    next_grid: Vec<bool>,
    /// Age of each living cell in generations
    age: Vec<u16>,
    /// Index into RULESETS
    ruleset_idx: usize,
    /// Timer for stepping generations
    step_timer: f64,
    /// Seconds between generations
    step_interval: f64,
    /// Timer for cycling rulesets
    cycle_timer: f64,
    /// Seconds before switching ruleset
    cycle_duration: f64,
    /// Generation counter for current ruleset
    generation: u64,
    rng: rand::rngs::ThreadRng,
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

/// Map cell age to a warm color: red -> orange -> yellow -> white.
fn age_to_color(age: u16) -> (u8, u8, u8) {
    let t = (age as f64 / MAX_AGE as f64).min(1.0);
    // Hue goes from 0.0 (red) through 0.08 (orange) to ~0.15 (yellow)
    // Saturation drops as cell ages (towards white)
    let hue = t * 0.16;
    let sat = 1.0 - t * 0.7;
    let val = 1.0;
    hsv_to_rgb(hue, sat, val)
}

impl Automata {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let size = width * height;
        let mut automata = Automata {
            width,
            height,
            grid: vec![false; size],
            next_grid: vec![false; size],
            age: vec![0; size],
            ruleset_idx: 0,
            step_timer: 0.0,
            step_interval: 0.08,
            cycle_timer: 0.0,
            cycle_duration: 17.0,
            generation: 0,
            rng: rand::rng(),
        };
        automata.seed_grid();
        automata
    }

    /// Fill the grid with a random pattern.
    fn seed_grid(&mut self) {
        let density: f64 = self.rng.random_range(0.15..0.45);
        for cell in self.grid.iter_mut() {
            *cell = self.rng.random_range(0.0..1.0) < density;
        }
        self.age.iter_mut().for_each(|a| *a = 0);
        // Set age=1 for initially alive cells so they get color immediately
        for i in 0..self.grid.len() {
            if self.grid[i] {
                self.age[i] = 1;
            }
        }
        self.generation = 0;
    }

    fn count_neighbors(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for dy in [-1i32, 0, 1] {
            for dx in [-1i32, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (x as i32 + dx).rem_euclid(self.width as i32) as usize;
                let ny = (y as i32 + dy).rem_euclid(self.height as i32) as usize;
                if self.grid[ny * self.width + nx] {
                    count += 1;
                }
            }
        }
        count
    }

    fn step(&mut self) {
        let ruleset = &RULESETS[self.ruleset_idx];

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let neighbors = self.count_neighbors(x, y);
                let alive = self.grid[idx];
                let next_alive = ruleset.next_state(alive, neighbors);
                self.next_grid[idx] = next_alive;

                if next_alive {
                    if alive {
                        // Cell survived: increment age
                        self.age[idx] = self.age[idx].saturating_add(1).min(MAX_AGE);
                    } else {
                        // Cell was just born
                        self.age[idx] = 1;
                    }
                } else {
                    self.age[idx] = 0;
                }
            }
        }

        // Swap buffers
        std::mem::swap(&mut self.grid, &mut self.next_grid);
        self.generation += 1;
    }

    fn switch_ruleset(&mut self) {
        self.ruleset_idx = (self.ruleset_idx + 1) % RULESETS.len();
        self.cycle_timer = 0.0;
        self.seed_grid();
    }
}

impl Animation for Automata {
    fn name(&self) -> &str {
        "automata"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::HalfBlock
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        *self = Automata::new(width, height, 1.0);
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Advance step timer
        self.step_timer += dt;
        while self.step_timer >= self.step_interval {
            self.step();
            self.step_timer -= self.step_interval;
        }

        // Advance cycle timer
        self.cycle_timer += dt;
        if self.cycle_timer >= self.cycle_duration {
            self.switch_ruleset();
        }

        // Also detect dead/stagnant grids and reset early
        let population: usize = self.grid.iter().filter(|&&c| c).count();
        if population == 0 && self.generation > 5 {
            self.switch_ruleset();
        }

        // Render
        canvas.clear();
        for y in 0..self.height.min(canvas.height) {
            let row = y * self.width;
            for x in 0..self.width.min(canvas.width) {
                let idx = row + x;
                if self.grid[idx] {
                    let (r, g, b) = age_to_color(self.age[idx]);
                    canvas.set_colored(x, y, 1.0, r, g, b);
                }
            }
        }
    }
}
