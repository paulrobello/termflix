# termflix Four Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement four independent features: macro-based animation registration, unified particle system, post-processing effects, and GIF export from recordings.

**Architecture:** Each feature touches different files with no cross-dependencies. Implemented sequentially in order of increasing complexity/risk.

**Tech Stack:** Rust 2024 edition, existing dependencies only (no new crates). GIF encoder hand-written (GIF89a + LZW).

---

## File Structure

| File | Feature | Change |
|------|---------|--------|
| `src/animations/mod.rs` | Macro | Replace manual lists with `declare_animations!` macro |
| `src/generators/mod.rs` | Particle | Extend `Particle` with RGB, add `emit_colored()`/`draw_colored()` |
| `src/animations/particles.rs` | Particle | Migrate from standalone to shared system |
| `src/render/canvas.rs` | PostProc | Add `PostProcessConfig` struct and `post_process()` method |
| `src/config.rs` | PostProc | Add `PostProcConfig` field, TOML parsing, config template |
| `src/main.rs` | PostProc + GIF | CLI flags, post_process() call, --export-gif handling |
| `src/gif.rs` | GIF | New file: GIF89a encoder, LZW compressor, ANSI decoder |
| `src/record.rs` | GIF | Expose frame data for GIF conversion |

---

## Task 1: Macro-Based Animation Registration

**Files:**
- Modify: `src/animations/mod.rs`

This is a pure refactor — no behavior change, verified by existing tests.

- [ ] **Step 1: Add the `declare_animations!` macro and replace manual registration**

Replace the manual `ANIMATIONS`, `ANIMATION_NAMES`, and `create()` function in `src/animations/mod.rs` with a single macro invocation. Keep all existing `pub mod` declarations and the `Animation` trait unchanged.

Add the macro definition and invocation after the `Animation` trait (around line 85). Remove the old `ANIMATIONS`, `ANIMATION_NAMES`, and `create()` function.

The macro uses a `$args:tt` token-tree capture for constructor arguments. Each entry specifies:
- `$name:literal` — string name ("fire")
- `$path:path` — type path (fire::Fire)
- `$desc:literal` — description string
- `$args:tt` — constructor arg pattern: `()`, `(w, h)`, or `(w, h, s)` where `w`=width, `h`=height, `s`=scale

```rust
macro_rules! declare_animations {
    ($(( $name:literal, $path:path, $desc:literal, $args:tt )),* $(,)?) => {
        pub const ANIMATIONS: &[(&str, &str)] = &[
            $( ($name, $desc), )*
        ];

        pub const ANIMATION_NAMES: &[&str] = &[
            $( $name, )*
        ];

        pub fn create(name: &str, width: usize, height: usize, scale: f64) -> Option<Box<dyn Animation>> {
            let (w, h, s) = (width, height, scale);
            Some(match name {
                $( $name => Box::new(<$path>::new $args), )*
                _ => return None,
            })
        }
    }
}

declare_animations! {
    ("fire", fire::Fire, "Doom-style fire effect with heat propagation", (w, h)),
    ("matrix", matrix::Matrix, "Matrix digital rain with trailing drops", (w, h, s)),
    ("plasma", plasma::Plasma, "Classic plasma with overlapping sine waves", ()),
    ("starfield", starfield::Starfield, "3D starfield with depth parallax", (w, h, s)),
    ("wave", wave::Wave, "Sine wave interference from moving sources", ()),
    ("life", life::GameOfLife, "Conway's Game of Life cellular automaton", (w, h)),
    ("particles", particles::Particles, "Fireworks bursting with physics and fade", (w, h, s)),
    ("pendulum", pendulum::Pendulum, "Pendulum wave with mesmerizing phase patterns", ()),
    ("rain", rain::Rain, "Raindrops with splash particles and wind", (w, h, s)),
    ("fountain", fountain::Fountain, "Water fountain with jets, splashes, and mist", (w, h, s)),
    ("flow", flow_field::FlowField, "Perlin noise flow field with particle trails", (w, h, s)),
    ("spiral", spiral::Spiral, "Rotating multi-arm spiral pattern", ()),
    ("ocean", ocean::Ocean, "Ocean waves with foam and depth shading", ()),
    ("aurora", aurora::Aurora, "Aurora borealis with layered curtains", ()),
    ("lightning", lightning::Lightning, "Lightning bolts with recursive branching", (w, h)),
    ("smoke", smoke::Smoke, "Smoke rising with Perlin turbulence", (w, h, s)),
    ("ripple", ripple::Ripple, "Ripple interference from random drop points", (w, h)),
    ("snow", snow::Snow, "Snowfall with accumulation on the ground", (w, h, s)),
    ("garden", garden::Garden, "Growing garden with rain, clouds, and blooming plants", (w, h, s)),
    ("fireflies", fireflies::Fireflies, "Fireflies blinking with warm glow", (w, h, s)),
    ("dna", dna::Dna, "Rotating DNA double helix with base pairs", ()),
    ("pulse", pulse::Pulse, "Expanding pulse rings from center", (w, h)),
    ("boids", boids::Boids, "Boids flocking simulation with trails", (w, h, s)),
    ("lava", lava::Lava, "Lava lamp blobs rising, merging, and splitting", (w, h, s)),
    ("sandstorm", sandstorm::Sandstorm, "Blowing sand with dune formation", (w, h, s)),
    ("petals", petals::Petals, "Cherry blossom petals drifting in wind", (w, h, s)),
    ("campfire", campfire::Campfire, "Campfire with rising ember sparks", (w, h, s)),
    ("waterfall", waterfall::Waterfall, "Cascading water with mist spray", (w, h, s)),
    ("eclipse", eclipse::Eclipse, "Moon crossing sun with corona rays", ()),
    ("blackhole", blackhole::Blackhole, "Black hole with accretion disk and lensing", ()),
    ("radar", radar::Radar, "Rotating radar sweep with fading blips", ()),
    ("rainforest", rainforest::Rainforest, "Layered rainforest with parallax scrolling, rain, birds, and falling leaves", (w, h, s)),
    ("crystallize", crystallize::Crystallize, "DLA crystal growth from center seed", (w, h, s)),
    ("hackerman", hackerman::Hackerman, "Scrolling hex/binary hacker terminal", (w, h, s)),
    ("visualizer", visualizer::Visualizer, "Audio spectrum analyzer with bouncing bars", (w, h, s)),
    ("cells", cells::Cells, "Cell division and mitosis animation", (w, h, s)),
    ("atom", atom::Atom, "Electrons orbiting a nucleus in 3D", ()),
    ("automata", automata::Automata, "Cellular automata cycling through multiple rulesets", (w, h, s)),
    ("globe", globe::Globe, "Rotating wireframe Earth with continents", ()),
    ("dragon", dragon::Dragon, "Dragon curve fractal with color cycling", ()),
    ("sierpinski", sierpinski::Sierpinski, "Animated Sierpinski triangle with zoom", ()),
    ("mandelbrot", mandelbrot::Mandelbrot, "Mandelbrot set with zoom and color cycling", ()),
    ("maze", maze::Maze, "Animated maze generation with recursive backtracking and BFS solving", (w, h, s)),
    ("metaballs", metaballs::Metaballs, "Organic metaballs merging and splitting with smooth distance fields", (w, h, s)),
    ("nbody", nbody::NBody, "N-body gravitational simulation with colorful orbiting masses and merging", (w, h, s)),
    ("langton", langton::Langton, "Langton's Ant cellular automaton", (w, h, s)),
    ("sort", sort::Sort, "Sorting algorithm visualizer", (w, h, s)),
    ("tetris", tetris::Tetris, "Self-playing Tetris with AI piece placement", (w, h, s)),
    ("snake", snake::Snake, "Self-playing Snake game AI", (w, h, s)),
    ("invaders", invaders::Invaders, "Space Invaders attract mode demo", (w, h, s)),
    ("pong", pong::Pong, "Self-playing Pong with AI paddles", (w, h, s)),
    ("flappy_bird", flappy_bird::FlappyBird, "Self-playing Flappy Bird with AI", (w, h, s)),
    ("reaction_diffusion", reaction_diffusion::ReactionDiffusion, "Gray-Scott reaction-diffusion coral/brain patterns", (w, h, s)),
    ("voronoi", voronoi::Voronoi, "Animated Voronoi diagram with drifting colored cells and edge detection", (w, h, s)),
}
```

- [ ] **Step 2: Run tests to verify no behavior change**

Run: `cargo test`
Expected: All existing tests pass, including:
- `test_create_returns_some_for_all_known_names`
- `test_animation_names_and_animations_have_same_length`
- `test_animation_names_match_animations_list`
- `test_created_animation_name_matches_requested`

Run: `cargo clippy --all-targets`
Expected: No warnings

- [ ] **Step 3: Commit**

```bash
git add src/animations/mod.rs
git commit -m "refactor: replace manual animation registration with declare_animations! macro"
```

---

## Task 2: Unified Particle System

**Files:**
- Modify: `src/generators/mod.rs`
- Modify: `src/animations/particles.rs`

- [ ] **Step 1: Write failing tests for per-particle color in generators**

Add tests to `src/generators/mod.rs` (at the bottom, inside the existing `#[cfg(test)]` module or add one):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_default_color_is_white() {
        let p = Particle {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
            life: 1.0, max_life: 1.0,
            r: 255, g: 255, b: 255,
        };
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 255);
        assert_eq!(p.b, 255);
    }

    #[test]
    fn test_emit_colored_sets_per_particle_color() {
        let config = EmitterConfig {
            x: 5.0, y: 5.0,
            spread: std::f64::consts::TAU,
            angle: 0.0,
            speed_min: 1.0, speed_max: 2.0,
            life_min: 1.0, life_max: 2.0,
            gravity: 0.0, drag: 1.0, wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop { t: 0.0, r: 255, g: 0, b: 0 },
                ColorStop { t: 1.0, r: 0, g: 0, b: 255 },
            ]),
        };
        let mut sys = ParticleSystem::new(config, 100);
        sys.emit_colored(10, (100, 200), (50, 150), (0, 50));
        assert_eq!(sys.particles.len(), 10);
        for p in &sys.particles {
            assert!((p.r as u8) >= 100 && (p.r as u8) <= 200);
            assert!((p.g as u8) >= 50 && (p.g as u8) <= 150);
            assert!((p.b as u8) >= 0 && (p.b as u8) <= 50);
        }
    }

    #[test]
    fn test_emit_colored_respects_capacity() {
        let config = EmitterConfig {
            x: 0.0, y: 0.0,
            spread: std::f64::consts::TAU,
            angle: 0.0,
            speed_min: 1.0, speed_max: 2.0,
            life_min: 1.0, life_max: 2.0,
            gravity: 0.0, drag: 1.0, wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop { t: 0.0, r: 255, g: 0, b: 0 },
                ColorStop { t: 1.0, r: 0, g: 0, b: 255 },
            ]),
        };
        let mut sys = ParticleSystem::new(config, 5);
        sys.emit_colored(10, (0, 255), (0, 255), (0, 255));
        assert_eq!(sys.particles.len(), 5);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test generators::tests`
Expected: Compilation errors — `emit_colored` does not exist yet, `Particle` doesn't have `r, g, b` fields.

- [ ] **Step 3: Extend `Particle` struct with RGB fields**

In `src/generators/mod.rs`, add `r`, `g`, `b` fields to `Particle`:

```rust
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub max_life: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
```

Update the `emit()` method to initialize `r: 255, g: 255, b: 255` for each new particle.

Update the `emit_at()` method to initialize `r: 255, g: 255, b: 255`.

- [ ] **Step 4: Add `emit_colored()` and `draw_colored()` methods to `ParticleSystem`**

In `src/generators/mod.rs`, add to `impl ParticleSystem`:

```rust
/// Emit particles with per-particle random color in the given ranges.
pub fn emit_colored(
    &mut self,
    count: usize,
    r_range: (u8, u8),
    g_range: (u8, u8),
    b_range: (u8, u8),
) {
    let mut rng = rand::rng();
    for _ in 0..count {
        if self.particles.len() >= self.capacity {
            break;
        }
        let half_spread = self.config.spread * 0.5;
        let angle = self.config.angle + rng.random_range(-half_spread..=half_spread);
        let speed = rng.random_range(self.config.speed_min..=self.config.speed_max);
        let life = rng.random_range(self.config.life_min..=self.config.life_max);
        let r = rng.random_range(r_range.0..=r_range.1);
        let g = rng.random_range(g_range.0..=g_range.1);
        let b = rng.random_range(b_range.0..=b_range.1);
        self.particles.push(Particle {
            x: self.config.x,
            y: self.config.y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            life,
            max_life: life,
            r, g, b,
        });
    }
}

/// Draw all particles using their per-particle color with life-based fade.
pub fn draw_colored(&self, canvas: &mut crate::render::Canvas) {
    for p in &self.particles {
        let ix = p.x as usize;
        let iy = p.y as usize;
        if ix < canvas.width && iy < canvas.height {
            let fade = p.life_frac();
            let r = (p.r as f64 * fade) as u8;
            let g = (p.g as f64 * fade) as u8;
            let b = (p.b as f64 * fade) as u8;
            canvas.set_colored(ix, iy, fade, r, g, b);
        }
    }
}
```

- [ ] **Step 5: Run generators tests to verify they pass**

Run: `cargo test generators`
Expected: All tests pass.

- [ ] **Step 6: Migrate `src/animations/particles.rs` to use shared system**

Replace the entire animation implementation to use `generators::ParticleSystem`:

```rust
use super::Animation;
use crate::generators::ParticleSystem;
use crate::render::Canvas;
use rand::RngExt;

/// Fireworks / particle fountain
pub struct Particles {
    width: usize,
    height: usize,
    system: ParticleSystem,
    spawn_timer: f64,
    gravity: f64,
    drag: f64,
}

impl Particles {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        use crate::generators::{EmitterConfig, ColorGradient, ColorStop};
        let config = EmitterConfig {
            x: 0.0,
            y: 0.0,
            spread: std::f64::consts::TAU,
            angle: 0.0,
            speed_min: 5.0,
            speed_max: 40.0,
            life_min: 0.8,
            life_max: 2.5,
            gravity: 15.0,
            drag: 0.99,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop { t: 0.0, r: 255, g: 255, b: 255 },
                ColorStop { t: 1.0, r: 255, g: 255, b: 255 },
            ]),
        };
        Particles {
            width,
            height,
            system: ParticleSystem::new(config, (2000.0 * scale) as usize),
            spawn_timer: 0.0,
            gravity: 15.0,
            drag: 0.99,
        }
    }
}

impl Animation for Particles {
    fn name(&self) -> &str {
        "particles"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(intensity) = params.intensity {
            self.gravity = intensity.clamp(0.0, 40.0);
        }
        if let Some(cs) = params.color_shift {
            self.drag = cs.clamp(0.9, 1.0);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("intensity", 0.0, 40.0), ("color_shift", 0.9, 1.0)]
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        // Spawn firework periodically
        self.spawn_timer += dt;
        if self.spawn_timer > 0.8 {
            self.spawn_timer = 0.0;
            let mut rng = rand::rng();
            let cx = rng.random_range(self.width as f64 * 0.2..self.width as f64 * 0.8);
            let cy = rng.random_range(self.height as f64 * 0.2..self.height as f64 * 0.6);
            let count = rng.random_range(30..80);
            let r: u8 = rng.random_range(100..255);
            let g: u8 = rng.random_range(100..255);
            let b: u8 = rng.random_range(100..255);

            // Set emitter position and emit colored particles
            self.system.config.x = cx;
            self.system.config.y = cy;
            self.system.emit_colored(count, (r, r), (g, g), (b, b));
        }

        // Apply gravity and drag to config (used by update)
        self.system.config.gravity = self.gravity;
        self.system.config.drag = self.drag;

        // Update physics
        self.system.update(dt);

        // Draw
        canvas.clear();
        self.system.draw_colored(canvas);
    }
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy --all-targets`
Expected: No warnings.

- [ ] **Step 8: Commit**

```bash
git add src/generators/mod.rs src/animations/particles.rs
git commit -m "refactor: unify particle system — extend shared Particle with per-particle color, migrate particles animation"
```

---

## Task 3: Post-Processing Effects

**Files:**
- Modify: `src/render/canvas.rs` — PostProcessConfig struct, post_process() method
- Modify: `src/config.rs` — config parsing, template
- Modify: `src/main.rs` — CLI flags, wire into render loop

- [ ] **Step 1: Write failing tests for post-processing passes**

Add to `src/render/canvas.rs` test module:

```rust
#[test]
fn test_bloom_brightens_neighbors_of_bright_pixel() {
    let mut c = test_canvas();
    // Set center pixel bright, neighbors dark
    let cx = 5;
    let cy = 5;
    c.pixels[cy * c.width + cx] = 0.9;
    c.apply_effects(1.0, 0.0);
    let cfg = PostProcessConfig { bloom: 0.5, vignette: 0.0, scanlines: false };
    c.post_process(&cfg);
    // Neighbors should be brighter than 0
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 { continue; }
            let nx = (cx as i32 + dx) as usize;
            let ny = (cy as i32 + dy) as usize;
            if nx < c.width && ny < c.height {
                assert!(c.pixels[ny * c.width + nx] > 0.0,
                    "neighbor ({}, {}) should be brightened by bloom", nx, ny);
            }
        }
    }
}

#[test]
fn test_vignette_darkens_edges() {
    let mut c = Canvas::new(10, 10, RenderMode::HalfBlock, ColorMode::TrueColor);
    // Fill all pixels bright
    for p in &mut c.pixels { *p = 1.0; }
    let cfg = PostProcessConfig { bloom: 0.0, vignette: 0.8, scanlines: false };
    c.post_process(&cfg);
    // Corner pixel should be darker than center
    let center = c.pixels[5 * c.width + 5];
    let corner = c.pixels[0];
    assert!(corner < center, "corner ({}) should be darker than center ({})", corner, center);
}

#[test]
fn test_scanlines_darkens_even_rows() {
    let mut c = test_canvas();
    for p in &mut c.pixels { *p = 1.0; }
    let cfg = PostProcessConfig { bloom: 0.0, vignette: 0.0, scanlines: true };
    c.post_process(&cfg);
    // Even rows should be dimmed, odd rows unchanged
    let even_val = c.pixels[0]; // row 0, col 0
    let odd_val = c.pixels[1 * c.width]; // row 1, col 0
    assert!(even_val < odd_val, "even row should be dimmed by scanlines");
    assert!((odd_val - 1.0).abs() < 1e-10, "odd row should be unchanged");
}

#[test]
fn test_post_process_noop_when_all_disabled() {
    let mut c = test_canvas();
    c.pixels[5 * c.width + 5] = 0.75;
    let before = c.pixels[5 * c.width + 5];
    let cfg = PostProcessConfig::default();
    c.post_process(&cfg);
    assert!((c.pixels[5 * c.width + 5] - before).abs() < 1e-10);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test render::canvas::tests`
Expected: Compilation error — `PostProcessConfig` does not exist.

- [ ] **Step 3: Add `PostProcessConfig` struct and `post_process()` method to `Canvas`**

In `src/render/canvas.rs`, add before `impl Canvas`:

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct PostProcessConfig {
    pub bloom: f64,
    pub vignette: f64,
    pub scanlines: bool,
}
```

Add to `impl Canvas`:

```rust
/// Apply post-processing effects to the canvas.
pub fn post_process(&mut self, config: &PostProcessConfig) {
    if config.bloom > 0.0 {
        self.apply_bloom(config.bloom);
    }
    if config.scanlines {
        self.apply_scanlines();
    }
    if config.vignette > 0.0 {
        self.apply_vignette(config.vignette);
    }
}

fn apply_bloom(&mut self, strength: f64) {
    let w = self.width;
    let h = self.height;
    let mut brightened = vec![0.0f64; w * h];
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if self.pixels[idx] > 0.6 {
                let boost = strength * 0.15 * self.pixels[idx];
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                            let nidx = ny as usize * w + nx as usize;
                            brightened[nidx] += boost;
                        }
                    }
                }
            }
        }
    }
    for i in 0..self.pixels.len() {
        self.pixels[i] = (self.pixels[i] + brightened[i]).clamp(0.0, 1.0);
    }
}

fn apply_vignette(&mut self, strength: f64) {
    let cx = self.width as f64 / 2.0;
    let cy = self.height as f64 / 2.0;
    let max_dist = (cx * cx + cy * cy).sqrt();
    if max_dist < 1e-10 { return; }
    for y in 0..self.height {
        for x in 0..self.width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / max_dist;
            let factor = 1.0 - (dist * dist * strength);
            let idx = y * self.width + x;
            self.pixels[idx] = (self.pixels[idx] * factor).clamp(0.0, 1.0);
        }
    }
}

fn apply_scanlines(&mut self) {
    for y in (0..self.height).step_by(2) {
        for x in 0..self.width {
            let idx = y * self.width + x;
            self.pixels[idx] *= 0.7;
        }
    }
}
```

- [ ] **Step 4: Run canvas tests to verify they pass**

Run: `cargo test render::canvas`
Expected: All tests pass.

- [ ] **Step 5: Add config support for post-processing**

In `src/config.rs`, add to `Config` struct (after `data_file`):

```rust
/// Post-processing effects
pub postproc: Option<PostProcConfig>,
```

Add the config type:

```rust
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct PostProcConfig {
    pub bloom: Option<f64>,
    pub vignette: Option<f64>,
    pub scanlines: Option<bool>,
}

impl Default for PostProcConfig {
    fn default() -> Self {
        PostProcConfig { bloom: None, vignette: None, scanlines: None }
    }
}
```

Add to `default_config_string()` (after the `# data_file` section):

```
# Post-processing effects
# [postproc]
# bloom = 0.3         # Glow effect (0.0-1.0)
# vignette = 0.4      # Edge darkening (0.0-1.0)
# scanlines = false   # CRT scanline effect
```

- [ ] **Step 6: Add CLI flags and wire into render loop**

In `src/main.rs`, add to `Cli` struct:

```rust
/// Bloom/glow post-processing effect intensity (0.0-1.0)
#[arg(long)]
bloom: Option<f64>,

/// Vignette edge-darkening intensity (0.0-1.0)
#[arg(long)]
vignette: Option<f64>,

/// Enable CRT scanline effect
#[arg(long)]
scanlines: bool,

/// Export recording to GIF (requires --play)
#[arg(long, value_name = "PATH")]
export_gif: Option<String>,
```

Add the import at the top of main.rs:

```rust
use render::{Canvas, ColorMode, PostProcessConfig, RenderMode};
```

In `run_loop()`, add `postproc` parameter:

```rust
fn run_loop(
    // ... existing params ...
    postproc: PostProcessConfig,
    // ... rest ...
)
```

After the `canvas.apply_effects(intensity, hue);` line (~line 634), add:

```rust
// Post-processing effects
canvas.post_process(&postproc);
```

In the `main()` function, build the `PostProcessConfig` and pass it to `run_loop()`:

```rust
let postproc = PostProcessConfig {
    bloom: cli.bloom.or(cfg.postproc.and_then(|p| p.bloom)).unwrap_or(0.0).clamp(0.0, 1.0),
    vignette: cli.vignette.or(cfg.postproc.and_then(|p| p.vignette)).unwrap_or(0.0).clamp(0.0, 1.0),
    scanlines: cli.scanlines || cfg.postproc.and_then(|p| p.scanlines).unwrap_or(false),
};
```

Update the `run_loop()` call to include `postproc`.

- [ ] **Step 7: Run all tests**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy --all-targets`
Expected: No warnings.

Run: `cargo build`
Expected: Clean build.

- [ ] **Step 8: Commit**

```bash
git add src/render/canvas.rs src/config.rs src/main.rs
git commit -m "feat: add post-processing effects (bloom, vignette, scanlines)"
```

---

## Task 4: GIF Export from Recordings

**Files:**
- Create: `src/gif.rs`
- Modify: `src/record.rs` — expose frame data
- Modify: `src/main.rs` — `--export-gif` handling

This is the largest task. The GIF encoder is ~400 LOC implementing GIF89a format with LZW compression and ANSI sequence decoding.

- [ ] **Step 1: Expose frame data from `record.rs`**

In `src/record.rs`, make `Frame` public and add accessor:

```rust
pub struct Frame {
    pub timestamp_ms: u64,
    pub content: String,
}
```

Add to `impl Player`:

```rust
/// Access recorded frames for export.
pub fn frames(&self) -> &[Frame] {
    &self.frames
}
```

Add `mod gif;` to `src/main.rs` at the top with the other module declarations.

- [ ] **Step 2: Create `src/gif.rs` with the GIF89a encoder**

The file structure:
1. Virtual terminal for ANSI decoding (struct `VirtualTerminal`)
2. Color quantization (6x7x6 uniform palette)
3. LZW compressor
4. GIF89a file writer (`GifEncoder`)

```rust
use std::io::{self, Write};

/// A cell in the virtual terminal.
#[derive(Clone)]
struct Cell {
    ch: char,
    r: u8,
    g: u8,
    b: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ch: ' ', r: 0, g: 0, b: 0 }
    }
}

/// Virtual terminal that processes ANSI escape sequences to build a character grid.
struct VirtualTerminal {
    cols: usize,
    rows: usize,
    grid: Vec<Cell>,
    cursor_row: usize,
    cursor_col: usize,
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
}

impl VirtualTerminal {
    fn new(cols: usize, rows: usize) -> Self {
        VirtualTerminal {
            cols,
            rows,
            grid: vec![Cell::default(); cols * rows],
            cursor_row: 0,
            cursor_col: 0,
            fg_r: 0,
            fg_g: 0,
            fg_b: 0,
        }
    }

    /// Process an ANSI-encoded frame and return the grid.
    fn process(&mut self, frame: &str) -> &[Cell] {
        // Reset state
        self.grid.fill(Cell::default());
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.fg_r = 0; self.fg_g = 0; self.fg_b = 0;

        let bytes = frame.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b {
                // Parse escape sequence
                i += 1;
                if i >= bytes.len() { break; }
                if bytes[i] == b'[' {
                    i += 1;
                    // Collect parameter bytes and intermediate bytes
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';' || bytes[i] == b'?') {
                        i += 1;
                    }
                    if i >= bytes.len() { break; }
                    let params = &frame.as_bytes()[start..i];
                    let params_str = std::str::from_utf8(params).unwrap_or("");
                    let cmd = bytes[i];
                    i += 1;

                    match cmd {
                        b'H' => {
                            // CUP - Cursor Position: row;col
                            let parts: Vec<&str> = params_str.split(';').collect();
                            if parts.len() >= 2 {
                                if let Ok(row) = parts[0].parse::<usize>() {
                                    self.cursor_row = row.saturating_sub(1);
                                }
                                if let Ok(col) = parts[1].parse::<usize>() {
                                    self.cursor_col = col.saturating_sub(1);
                                }
                            }
                        }
                        b'm' => {
                            // SGR - Set Graphics Rendition
                            self.parse_sgr(params_str);
                        }
                        // Ignore: 'l' (reset mode), 'h' (set mode), 'K' (erase line), etc.
                        _ => {}
                    }
                }
            } else {
                // Regular character
                let ch = frame[i..].chars().next().unwrap_or(' ');
                if ch == '\n' {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                } else if ch != '\r' {
                    if self.cursor_row < self.rows && self.cursor_col < self.cols {
                        let idx = self.cursor_row * self.cols + self.cursor_col;
                        self.grid[idx] = Cell {
                            ch,
                            r: self.fg_r,
                            g: self.fg_g,
                            b: self.fg_b,
                        };
                    }
                    self.cursor_col += 1;
                }
                i += ch.len_utf8();
            }
        }
        &self.grid
    }

    fn parse_sgr(&mut self, params: &str) {
        if params.is_empty() || params == "0" {
            // Reset
            self.fg_r = 0; self.fg_g = 0; self.fg_b = 0;
            return;
        }
        let parts: Vec<&str> = params.split(';').collect();
        if parts.len() >= 1 && parts[0] == "38" {
            if parts.len() >= 5 && parts[1] == "2" {
                // 38;2;r;g;b — truecolor
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[2].parse::<u8>(),
                    parts[3].parse::<u8>(),
                    parts[4].parse::<u8>(),
                ) {
                    self.fg_r = r; self.fg_g = g; self.fg_b = b;
                }
            } else if parts.len() >= 3 && parts[1] == "5" {
                // 38;5;N — 256-color
                if let Ok(idx) = parts[2].parse::<u8>() {
                    let (r, g, b) = ansi256_to_rgb(idx);
                    self.fg_r = r; self.fg_g = g; self.fg_b = b;
                }
            }
        }
        // 48;... — background, ignore for GIF (we use dark bg)
    }
}

/// Convert 256-color index to RGB.
fn ansi256_to_rgb(idx: u8) -> (u8, u8, u8) {
    if idx < 16 {
        // Standard 16 colors
        const TABLE: [(u8, u8, u8); 16] = [
            (0,0,0), (128,0,0), (0,128,0), (128,128,0),
            (0,0,128), (128,0,128), (0,128,128), (192,192,192),
            (128,128,128), (255,0,0), (0,255,0), (255,255,0),
            (0,0,255), (255,0,255), (0,255,255), (255,255,255),
        ];
        TABLE[idx as usize]
    } else if idx < 232 {
        // 6x6x6 color cube
        let idx = idx - 16;
        let b = idx % 6;
        let g = (idx / 6) % 6;
        let r = idx / 36;
        let scale = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
        (scale(r), scale(g), scale(b))
    } else {
        // Grayscale
        let v = 8 + (idx - 232) * 10;
        (v, v, v)
    }
}

// --- Color Quantization (6x7x6 = 252 colors) ---

const PALETTE_R_LEVELS: u8 = 6;
const PALETTE_G_LEVELS: u8 = 7;
const PALETTE_B_LEVELS: u8 = 6;
const PALETTE_SIZE: usize = (PALETTE_R_LEVELS as usize) * (PALETTE_G_LEVELS as usize) * (PALETTE_B_LEVELS as usize);

fn build_palette() -> Vec<(u8, u8, u8)> {
    let mut palette = Vec::with_capacity(PALETTE_SIZE + 4);
    for ri in 0..PALETTE_R_LEVELS {
        for gi in 0..PALETTE_G_LEVELS {
            for bi in 0..PALETTE_B_LEVELS {
                let r = if PALETTE_R_LEVELS > 1 { (ri as u16 * 255 / (PALETTE_R_LEVELS as u16 - 1)) as u8 } else { 0 };
                let g = if PALETTE_G_LEVELS > 1 { (gi as u16 * 255 / (PALETTE_G_LEVELS as u16 - 1)) as u8 } else { 0 };
                let b = if PALETTE_B_LEVELS > 1 { (bi as u16 * 255 / (PALETTE_B_LEVELS as u16 - 1)) as u8 } else { 0 };
                palette.push((r, g, b));
            }
        }
    }
    // Add black and white for safety
    palette.push((0, 0, 0));
    palette.push((255, 255, 255));
    palette.push((128, 128, 128));
    palette.push((64, 64, 64));
    palette
}

fn quantize(r: u8, g: u8, b: u8, palette: &[(u8, u8, u8)]) -> u8 {
    let mut best_idx = 0u8;
    let mut best_dist = u32::MAX;
    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u8;
        }
    }
    best_idx
}

// --- LZW Compressor ---

struct LzwEncoder {
    min_code_size: u8,
    buf: u8,
    bits_in_buf: u8,
    output: Vec<u8>,
}

impl LzwEncoder {
    fn new(min_code_size: u8) -> Self {
        LzwEncoder {
            min_code_size,
            buf: 0,
            bits_in_buf: 0,
            output: Vec::new(),
        }
    }

    fn encode(&mut self, indices: &[u8]) {
        let clear_code = 1u16 << self.min_code_size;
        let eoi_code = clear_code + 1;
        let mut code_size = self.min_code_size + 1;
        let mut next_code = eoi_code + 1;
        let max_code = 4096u16;

        let mut table: std::collections::HashMap<Vec<u8>, u16> = std::collections::HashMap::new();
        for i in 0..(1u16 << self.min_code_size) {
            table.insert(vec![i as u8], i as u16);
        }

        self.write_bits(clear_code, code_size);

        if indices.is_empty() {
            self.write_bits(eoi_code, code_size);
            self.flush_bits();
            return;
        }

        let mut current = vec![indices[0]];
        for &byte in &indices[1..] {
            let mut candidate = current.clone();
            candidate.push(byte);
            if table.contains_key(&candidate) {
                current = candidate;
            } else {
                if let Some(&code) = table.get(&current) {
                    self.write_bits(code, code_size);
                }
                if next_code < max_code {
                    table.insert(candidate, next_code);
                    next_code += 1;
                    if next_code > (1u16 << code_size) && code_size < 12 {
                        code_size += 1;
                    }
                } else {
                    self.write_bits(clear_code, code_size);
                    table.clear();
                    for i in 0..(1u16 << self.min_code_size) {
                        table.insert(vec![i as u8], i as u16);
                    }
                    code_size = self.min_code_size + 1;
                    next_code = eoi_code + 1;
                }
                current = vec![byte];
            }
        }
        if let Some(&code) = table.get(&current) {
            self.write_bits(code, code_size);
        }
        self.write_bits(eoi_code, code_size);
        self.flush_bits();
    }

    fn write_bits(&mut self, code: u16, nbits: u8) {
        let mut code = code;
        let mut remaining = nbits;
        while remaining > 0 {
            let bits_to_write = remaining.min(8 - self.bits_in_buf);
            self.buf |= ((code & ((1 << bits_to_write) - 1)) as u8) << self.bits_in_buf;
            self.bits_in_buf += bits_to_write;
            code >>= bits_to_write;
            remaining -= bits_to_write;
            if self.bits_in_buf == 8 {
                self.output.push(self.buf);
                self.buf = 0;
                self.bits_in_buf = 0;
            }
        }
    }

    fn flush_bits(&mut self) {
        if self.bits_in_buf > 0 {
            self.output.push(self.buf);
            self.buf = 0;
            self.bits_in_buf = 0;
        }
    }
}

// --- GIF89a Writer ---

/// Export a recording to GIF.
pub fn export_gif<W: Write>(
    writer: &mut W,
    frames: &[(u64, String)], // (timestamp_ms, ansi_content)
    term_cols: usize,
    term_rows: usize,
) -> io::Result<()> {
    if frames.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No frames to export"));
    }

    let width = term_cols as u16;
    let height = term_rows as u16;
    let palette = build_palette();
    let bg_index = quantize(0, 0, 0, &palette) as u8;

    // Header
    writer.write_all(b"GIF89a")?;

    // Logical Screen Descriptor
    writer.write_all(&width.to_le_bytes())?;
    writer.write_all(&height.to_le_bytes())?;
    // GCT flag=1, color resolution=7 (8 bits), sort=0, GCT size = ceil(log2(palette len)) - 1
    let gct_size_bits = palette.len().next_power_of_two().trailing_zeros() as u8 - 1;
    writer.write_all(&[0x80 | (7 << 4) | gct_size_bits])?; // packed field
    writer.write_all(&[bg_index])?; // background color index
    writer.write_all(&[0])?; // pixel aspect ratio

    // Global Color Table
    let gct_entries = 1 << (gct_size_bits + 1);
    for &(r, g, b) in &palette {
        writer.write_all(&[r, g, b])?;
    }
    // Pad remaining entries with black
    for _ in palette.len()..gct_entries {
        writer.write_all(&[0, 0, 0])?;
    }

    // NETSCAPE2.0 Application Extension (loop forever)
    writer.write_all(&[0x21, 0xFF, 11])?;
    writer.write_all(b"NETSCAPE2.0")?;
    writer.write_all(&[3, 1, 0, 0, 0])?; // sub-block: loop count = 0 (infinite)

    // Process frames
    let mut vt = VirtualTerminal::new(term_cols, term_rows);
    let mut prev_indices: Vec<u8> = vec![bg_index; term_cols * term_rows];

    for (i, (timestamp_ms, content)) in frames.iter().enumerate() {
        let grid = vt.process(content);

        // Build index array for this frame
        let mut indices = Vec::with_capacity(term_cols * term_rows);
        for cell in grid {
            if cell.ch == ' ' {
                indices.push(bg_index);
            } else {
                indices.push(quantize(cell.r, cell.g, cell.b, &palette));
            }
        }

        // Frame deduplication: skip if identical to previous
        if indices == prev_indices && i > 0 {
            continue;
        }
        prev_indices = indices.clone();

        // Compute delay in centiseconds
        let delay_cs = if i + 1 < frames.len() {
            let next_ts = frames[i + 1].0;
            let delta = next_ts.saturating_sub(*timestamp_ms);
            ((delta as f64 / 10.0).round() as u16).max(2)
        } else {
            10 // 100ms default for last frame
        };

        // Graphic Control Extension
        writer.write_all(&[0x21, 0xF9, 4])?;
        writer.write_all(&[0x00])?; // disposal method: none
        writer.write_all(&delay_cs.to_le_bytes())?;
        writer.write_all(&[0, 0])?; // transparent color index (none)

        // Image Descriptor
        writer.write_all(&[0x2C])?;
        writer.write_all(&[0, 0])?; // left
        writer.write_all(&[0, 0])?; // top
        writer.write_all(&width.to_le_bytes())?;
        writer.write_all(&height.to_le_bytes())?;
        writer.write_all(&[0])?; // no local color table

        // Image Data (LZW)
        let min_code_size = 8u8.min(gct_size_bits + 1);
        writer.write_all(&[min_code_size])?;
        let mut encoder = LzwEncoder::new(min_code_size);
        encoder.encode(&indices);
        // Write sub-blocks (max 255 bytes each)
        let compressed = &encoder.output;
        let mut pos = 0;
        while pos < compressed.len() {
            let chunk_len = (compressed.len() - pos).min(255);
            writer.write_all(&[chunk_len as u8])?;
            writer.write_all(&compressed[pos..pos + chunk_len])?;
            pos += chunk_len;
        }
        writer.write_all(&[0])?; // block terminator
    }

    // Trailer
    writer.write_all(&[0x3B])?;
    Ok(())
}
```

- [ ] **Step 3: Wire `--export-gif` into `src/main.rs`**

In `main()`, before the `play` block (~line 122), add GIF export handling:

```rust
if let Some(ref path) = cli.play {
    if let Some(ref gif_path) = cli.export_gif {
        // Export to GIF instead of playing
        let player = record::Player::load(path)?;
        if player.frames().is_empty() {
            eprintln!("No frames to export.");
            std::process::exit(1);
        }
        // Determine terminal size from the first frame
        // Parse to find grid dimensions (use 80x24 as default, or detect from frame content)
        let (cols, rows) = detect_recording_size(player.frames());
        let file = std::fs::File::create(gif_path)?;
        let mut writer = std::io::BufWriter::new(file);
        match gif::export_gif(&mut writer, player.frames_as_tuples(), cols, rows) {
            Ok(()) => {
                println!("Exported {} frames to {}", player.frames().len(), gif_path);
            }
            Err(e) => {
                eprintln!("GIF export failed: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let player = record::Player::load(path)?;
    return player.play();
}
```

Add a helper function to detect recording size:

```rust
fn detect_recording_size(frames: &[record::Frame]) -> (usize, usize) {
    // Scan first frame for cursor positioning to determine dimensions
    // Default to 80x24
    let mut max_row = 24usize;
    let mut max_col = 80usize;
    if let Some(frame) = frames.first() {
        let bytes = frame.content.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';') {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'H' {
                    let params = &frame.content.as_bytes()[start..i];
                    let s = std::str::from_utf8(params).unwrap_or("1;1");
                    let parts: Vec<&str> = s.split(';').collect();
                    if parts.len() >= 2 {
                        if let Ok(r) = parts[0].parse::<usize>() { max_row = max_row.max(r); }
                        if let Ok(c) = parts[1].parse::<usize>() { max_col = max_col.max(c); }
                    }
                }
                i += 1;
            } else {
                i += 1;
            }
        }
    }
    (max_col, max_row)
}
```

- [ ] **Step 4: Add `frames_as_tuples` method to `Player`**

In `src/record.rs`, add to `impl Player`:

```rust
/// Get frames as (timestamp_ms, content) tuples for GIF export.
pub fn frames_as_tuples(&self) -> Vec<(u64, String)> {
    self.frames.iter().map(|f| (f.timestamp_ms, f.content.clone())).collect()
}
```

Actually, the `export_gif` function takes `&[(u64, String)]`. Instead of creating tuples, change the `export_gif` signature to take `&[Frame]` and access fields directly. This avoids the allocation:

```rust
pub fn export_gif<W: Write>(
    writer: &mut W,
    frames: &[crate::record::Frame],
    term_cols: usize,
    term_rows: usize,
) -> io::Result<()> {
```

Then in the loop use `frame.timestamp_ms` and `&frame.content`. Update the `Player` method:

```rust
/// Access frames for export.
pub fn frames(&self) -> &[Frame] {
    &self.frames
}
```

And in main.rs, call:
```rust
gif::export_gif(&mut writer, player.frames(), cols, rows)?;
```

- [ ] **Step 5: Run all tests and fix any issues**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy --all-targets`
Expected: No warnings.

Run: `cargo build`
Expected: Clean build.

- [ ] **Step 6: Commit**

```bash
git add src/gif.rs src/record.rs src/main.rs
git commit -m "feat: add GIF export from .asciianim recordings with --export-gif flag"
```

---

## Task 5: Final Verification and Cleanup

- [ ] **Step 1: Run full verification suite**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

All must pass with no errors or warnings.

- [ ] **Step 2: Update ideas.md — mark completed items**

Mark these items as completed in `ideas.md`:
- `[render] Post-Processing Effects`
- `[record] GIF/APNG Export`
- `[arch] Macro-Based Animation Registration`
- `[arch] Unified Particle System`

- [ ] **Step 3: Update changelog / version if needed**

Update version in `Cargo.toml` from `0.4.2` to `0.5.0` (minor bump for new features).

- [ ] **Step 4: Final commit**

```bash
git add ideas.md Cargo.toml
git commit -m "docs: update ideas.md, bump version to 0.5.0"
```
