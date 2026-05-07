# Four Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove vortex dead code, expose external params on 6 animations, add fade transitions between animations, and add configurable keybindings.

**Architecture:** Four independent features touching different files. Dead code cleanup deletes one file. Param exposure adds trait method overrides to 6 animation files. Transitions add a state machine to the main loop. Keybindings add config parsing and event matching in main.rs.

**Tech Stack:** Rust, crossterm 0.29, serde/toml, clap

---

## Task 1: Remove vortex dead code

**Files:**
- Delete: `src/animations/vortex.rs`

- [ ] **Step 1: Delete vortex.rs**

```bash
rm src/animations/vortex.rs
```

- [ ] **Step 2: Verify build and tests pass**

Run: `cargo test --quiet 2>&1`
Expected: All tests pass. The file was never compiled (no `pub mod vortex;` declaration), so nothing references it.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: remove dead vortex animation (was never registered)"
```

---

## Task 2: Expose params on boids

**Files:**
- Modify: `src/animations/boids.rs`

Boids has hardcoded force factors at lines 170-172: `sep_factor = 2.0`, `align_factor = 0.05`, `cohes_factor = 0.005`. We'll make `cohes_factor` adjustable via `intensity` and `sep_factor` via `color_shift`, storing them as struct fields.

- [ ] **Step 1: Add fields to Boids struct**

Replace the Boids struct (lines 67-72) with:

```rust
pub struct Boids {
    width: usize,
    height: usize,
    boids: Vec<Boid>,
    grid: SpatialGrid,
    cohes_factor: f64,
    sep_factor: f64,
}
```

- [ ] **Step 2: Update constructor to initialize new fields**

In `Boids::new()` (line 75), update the return value:

```rust
        Boids {
            width,
            height,
            boids,
            grid: SpatialGrid::new(width as f64, height as f64, 25.0),
            cohes_factor: 0.005,
            sep_factor: 2.0,
        }
```

- [ ] **Step 3: Update on_resize to preserve new fields**

Replace the `on_resize` method (lines 106-110) with:

```rust
    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.grid = SpatialGrid::new(width as f64, height as f64, 25.0);
    }
```

(This is unchanged — just confirming it doesn't touch the new fields.)

- [ ] **Step 4: Use struct fields instead of local constants in update**

In `update()` (lines 170-172), replace the three hardcoded constants:

```rust
            let sep_factor = 2.0;
            let align_factor = 0.05;
            let cohes_factor = 0.005;
```

with:

```rust
            let sep_factor = self.sep_factor;
            let align_factor = 0.05;
            let cohes_factor = self.cohes_factor;
```

- [ ] **Step 5: Add set_params and supported_params trait methods**

Add these two methods after `on_resize` and before `update`:

```rust
    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(intensity) = params.intensity {
            self.cohes_factor = intensity.clamp(0.001, 0.05);
        }
        if let Some(cs) = params.color_shift {
            self.sep_factor = cs.clamp(0.5, 5.0);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("intensity", 0.001, 0.05), ("color_shift", 0.5, 5.0)]
    }
```

- [ ] **Step 6: Verify build and tests**

Run: `cargo test --quiet 2>&1`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/animations/boids.rs
git commit -m "feat(boids): expose cohesion and separation as external params"
```

---

## Task 3: Expose params on particles

**Files:**
- Modify: `src/animations/particles.rs`

Particles has gravity hardcoded at line 94 (`p.vy += 15.0 * dt`) and drag at line 95 (`p.vx *= 0.99`). We'll make gravity adjustable via `intensity` and drag via `color_shift`.

- [ ] **Step 1: Add fields to Particles struct**

Replace the struct (lines 18-24) with:

```rust
pub struct Particles {
    width: usize,
    height: usize,
    particles: Vec<Particle>,
    spawn_timer: f64,
    rng: rand::rngs::ThreadRng,
    gravity: f64,
    drag: f64,
}
```

- [ ] **Step 2: Update constructor**

In `Particles::new()` (lines 27-34), update the return value:

```rust
        Particles {
            width,
            height,
            particles: Vec::with_capacity((2000.0 * scale) as usize),
            spawn_timer: 0.0,
            rng: rand::rng(),
            gravity: 15.0,
            drag: 0.99,
        }
```

- [ ] **Step 3: Use struct fields in update**

In `update()`, replace line 94:

```rust
            p.vy += 15.0 * dt; // gravity
```

with:

```rust
            p.vy += self.gravity * dt;
```

Replace line 95:

```rust
            p.vx *= 0.99; // drag
```

with:

```rust
            p.vx *= self.drag;
```

- [ ] **Step 4: Add set_params and supported_params**

Add these methods after `on_resize` and before `update`:

```rust
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
```

- [ ] **Step 5: Verify build and tests**

Run: `cargo test --quiet 2>&1`

- [ ] **Step 6: Commit**

```bash
git add src/animations/particles.rs
git commit -m "feat(particles): expose gravity and drag as external params"
```

---

## Task 4: Expose params on wave

**Files:**
- Modify: `src/animations/wave.rs`

Wave has frequency multipliers hardcoded at lines 37-38 (`0.3` in the sine arguments) and implicit amplitude from the `* 0.5` normalization at line 39. We'll make amplitude adjustable via `intensity` and frequency via `color_shift`.

- [ ] **Step 1: Replace unit struct with field struct**

Replace the entire `Wave` struct and `impl Wave` (lines 4-11) with:

```rust
/// Sine wave interference pattern
pub struct Wave {
    amplitude: f64,
    frequency: f64,
}

impl Wave {
    pub fn new() -> Self {
        Wave {
            amplitude: 0.5,
            frequency: 0.3,
        }
    }
}
```

- [ ] **Step 2: Use struct fields in update**

In `update()`, replace lines 37-39:

```rust
                let wave1 = (d1 * 0.3 - t * 4.0).sin();
                let wave2 = (d2 * 0.3 - t * 3.5).sin();
                let combined = (wave1 + wave2) * 0.5;
```

with:

```rust
                let wave1 = (d1 * self.frequency - t * 4.0).sin();
                let wave2 = (d2 * self.frequency - t * 3.5).sin();
                let combined = (wave1 + wave2) * self.amplitude;
```

- [ ] **Step 3: Add set_params and supported_params**

Add these methods before `update`:

```rust
    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(intensity) = params.intensity {
            self.amplitude = intensity.clamp(0.1, 1.0);
        }
        if let Some(cs) = params.color_shift {
            self.frequency = cs.clamp(0.05, 0.8);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("intensity", 0.1, 1.0), ("color_shift", 0.05, 0.8)]
    }
```

- [ ] **Step 4: Verify build and tests**

Run: `cargo test --quiet 2>&1`

- [ ] **Step 5: Commit**

```bash
git add src/animations/wave.rs
git commit -m "feat(wave): expose amplitude and frequency as external params"
```

---

## Task 5: Expose params on sort

**Files:**
- Modify: `src/animations/sort.rs`

Sort has `ops_per_frame: usize` at line 41, initialized to `3` at line 65. This controls how many sort operations happen per frame, effectively controlling visual speed. We'll expose this via `speed`.

- [ ] **Step 1: Add set_params and supported_params**

Add these methods to the `impl Animation for Sort` block, before `update` (before line 218):

```rust
    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(speed) = params.speed {
            self.ops_per_frame = speed.clamp(1.0, 20.0) as usize;
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("speed", 1.0, 20.0)]
    }
```

- [ ] **Step 2: Verify build and tests**

Run: `cargo test --quiet 2>&1`

- [ ] **Step 3: Commit**

```bash
git add src/animations/sort.rs
git commit -m "feat(sort): expose visual speed as external param"
```

---

## Task 6: Expose params on snake and pong

**Files:**
- Modify: `src/animations/snake.rs`
- Modify: `src/animations/pong.rs`

Snake has `move_interval: f64` at line 48, initialized to `0.08` at line 74. Pong has no explicit tick rate (ball moves every frame via `ball_x += ball_vx * dt`), so we'll expose ball speed via `speed`.

**Snake changes:**

- [ ] **Step 1: Add set_params and supported_params to Snake**

Add these methods to the `impl Animation for Snake` block, before `update` (before line 153):

```rust
    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(speed) = params.speed {
            self.move_interval = speed.clamp(0.02, 0.2);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("speed", 0.02, 0.2)]
    }
```

- [ ] **Step 2: Add set_params and supported_params to Pong**

Add these methods to the `impl Animation for Pong` block, before `update` (before line 64). Also add a `speed_mult` field. First, add the field to the struct (lines 6-18):

Replace struct with:

```rust
pub struct Pong {
    ball_x: f64,
    ball_y: f64,
    ball_vx: f64,
    ball_vy: f64,
    left_y: f64,
    right_y: f64,
    paddle_h: f64,
    left_score: u32,
    right_score: u32,
    serve_timer: f64,
    speed_mult: f64,
    rng: rand::rngs::ThreadRng,
}
```

In `Pong::new()` (lines 21-43), add `speed_mult: 1.0,` to the struct initialization.

In `update()`, scale ball movement. Replace lines 79-80:

```rust
            self.ball_x += self.ball_vx * dt;
            self.ball_y += self.ball_vy * dt;
```

with:

```rust
            self.ball_x += self.ball_vx * self.speed_mult * dt;
            self.ball_y += self.ball_vy * self.speed_mult * dt;
```

Add the trait methods before `update`:

```rust
    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        if let Some(speed) = params.speed {
            self.speed_mult = speed.clamp(0.2, 3.0);
        }
    }

    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[("speed", 0.2, 3.0)]
    }
```

- [ ] **Step 3: Verify build and tests**

Run: `cargo test --quiet 2>&1`

- [ ] **Step 4: Commit**

```bash
git add src/animations/snake.rs src/animations/pong.rs
git commit -m "feat(snake,pong): expose game speed as external param"
```

---

## Task 7: Add tests for newly exposed params

**Files:**
- Modify: `src/animations/mod.rs`

Add test cases for each newly parameterized animation, following the existing pattern at lines 330-343.

- [ ] **Step 1: Add tests for each animation**

Add these tests at the end of the `tests` module (after line 351):

```rust
    #[test]
    fn test_boids_supported_params() {
        let anim = create("boids", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
        assert!(params.iter().any(|&(name, _, _)| name == "color_shift"));
    }

    #[test]
    fn test_particles_supported_params() {
        let anim = create("particles", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
    }

    #[test]
    fn test_wave_supported_params() {
        let anim = create("wave", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
        assert!(params.iter().any(|&(name, _, _)| name == "color_shift"));
    }

    #[test]
    fn test_sort_supported_params() {
        let anim = create("sort", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "speed"));
    }

    #[test]
    fn test_snake_supported_params() {
        let anim = create("snake", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "speed"));
    }

    #[test]
    fn test_pong_supported_params() {
        let anim = create("pong", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "speed"));
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test --quiet 2>&1`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/animations/mod.rs
git commit -m "test: add supported_params tests for boids, particles, wave, sort, snake, pong"
```

---

## Task 8: Add transition effects between animations

**Files:**
- Modify: `src/main.rs`

This is the most complex task. We add a `TransitionState` enum and integrate it into the main loop. The key insight: we reuse the existing `canvas.apply_effects(intensity, hue)` pipeline by multiplying the intensity by a transition factor.

- [ ] **Step 1: Add TransitionState enum**

Add this enum in `main.rs` just before `run_loop` (around line 288):

```rust
const TRANSITION_FRAMES: u8 = 8;

enum TransitionState {
    None,
    FadingOut {
        next_anim_index: usize,
        remaining: u8,
    },
    FadingIn {
        remaining: u8,
    },
}
```

- [ ] **Step 2: Add transition state variable in run_loop**

In `run_loop`, after the `ext_state` initialization (after line 358), add:

```rust
    let mut transition = TransitionState::None;
```

- [ ] **Step 3: Create a helper function to start a transition**

Add this helper function after the `TransitionState` enum:

```rust
fn start_transition(transition: &mut TransitionState, next_anim_index: usize) {
    // If already fading out to a different target, update target
    *transition = TransitionState::FadingOut {
        next_anim_index,
        remaining: TRANSITION_FRAMES,
    };
}
```

- [ ] **Step 4: Replace animation switch code in hotkey handlers**

In the `KeyCode::Right | KeyCode::Char('n')` handler (lines 399-413), replace the animation creation block with a transition trigger. Replace:

```rust
                            KeyCode::Right | KeyCode::Char('n') => {
                                anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
                                anim = animations::create(
                                    animations::ANIMATION_NAMES[anim_index],
                                    canvas.width,
                                    canvas.height,
                                    scale,
                                )
                                .expect("animation name validated before calling create");
                                anim.on_resize(canvas.width, canvas.height);
                                if explicit_render.is_none() {
                                    render_mode = anim.preferred_render();
                                    needs_rebuild = true;
                                }
                                cycle_start = Instant::now();
                            }
```

with:

```rust
                            KeyCode::Right | KeyCode::Char('n') => {
                                anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
                                start_transition(&mut transition, anim_index);
                                cycle_start = Instant::now();
                            }
```

Do the same for `KeyCode::Left | KeyCode::Char('p')` (lines 415-433), replacing the animation creation block with:

```rust
                            KeyCode::Left | KeyCode::Char('p') => {
                                anim_index = if anim_index == 0 {
                                    animations::ANIMATION_NAMES.len() - 1
                                } else {
                                    anim_index - 1
                                };
                                start_transition(&mut transition, anim_index);
                                cycle_start = Instant::now();
                            }
```

- [ ] **Step 5: Replace animation switch in auto-cycle**

Replace the auto-cycle block (lines 508-523):

```rust
        if cycle > 0 && cycle_start.elapsed() >= Duration::from_secs(cycle as u64) {
            anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
            anim = animations::create(
                animations::ANIMATION_NAMES[anim_index],
                canvas.width,
                canvas.height,
                scale,
            )
            .expect("animation name validated before calling create");
            anim.on_resize(canvas.width, canvas.height);
            if explicit_render.is_none() {
                render_mode = anim.preferred_render();
                needs_rebuild = true;
            }
            cycle_start = Instant::now();
        }
```

with:

```rust
        if cycle > 0 && cycle_start.elapsed() >= Duration::from_secs(cycle as u64) {
            anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
            start_transition(&mut transition, anim_index);
            cycle_start = Instant::now();
        }
```

- [ ] **Step 6: Replace animation switch in external params handler**

Replace the external animation change block (lines 538-559):

```rust
        if let Some(name) = ext_state.take_animation_change()
            && animations::ANIMATION_NAMES.contains(&name.as_str())
        {
            anim_index = animations::ANIMATION_NAMES
                .iter()
                .position(|&n| n == name.as_str())
                .unwrap_or(anim_index);
            anim = animations::create(
                animations::ANIMATION_NAMES[anim_index],
                canvas.width,
                canvas.height,
                scale,
            )
            .expect("animation name validated before calling create");
            anim.on_resize(canvas.width, canvas.height);
            if explicit_render.is_none() {
                render_mode = anim.preferred_render();
                needs_rebuild = true;
            }
            cycle_start = Instant::now();
        }
```

with:

```rust
        if let Some(name) = ext_state.take_animation_change()
            && animations::ANIMATION_NAMES.contains(&name.as_str())
        {
            anim_index = animations::ANIMATION_NAMES
                .iter()
                .position(|&n| n == name.as_str())
                .unwrap_or(anim_index);
            start_transition(&mut transition, anim_index);
            cycle_start = Instant::now();
        }
```

- [ ] **Step 7: Add transition frame processing before canvas.apply_effects**

Just before the `canvas.apply_effects(intensity, hue)` line (line 609), add transition processing:

```rust
        // Transition fade processing
        let transition_factor = match &mut transition {
            TransitionState::None => 1.0,
            TransitionState::FadingOut {
                next_anim_index,
                remaining,
            } => {
                let factor = *remaining as f64 / TRANSITION_FRAMES as f64;
                if *remaining == 0 {
                    // Switch animation
                    anim = animations::create(
                        animations::ANIMATION_NAMES[*next_anim_index],
                        canvas.width,
                        canvas.height,
                        scale,
                    )
                    .expect("animation name validated before calling create");
                    anim.on_resize(canvas.width, canvas.height);
                    if explicit_render.is_none() {
                        render_mode = anim.preferred_render();
                        needs_rebuild = true;
                    }
                    transition = TransitionState::FadingIn {
                        remaining: TRANSITION_FRAMES,
                    };
                    0.0
                } else {
                    *remaining -= 1;
                    factor
                }
            }
            TransitionState::FadingIn { remaining } => {
                let factor = 1.0 - *remaining as f64 / TRANSITION_FRAMES as f64;
                if *remaining == 0 {
                    transition = TransitionState::None;
                    1.0
                } else {
                    *remaining -= 1;
                    factor
                }
            }
        };
```

- [ ] **Step 8: Apply transition factor to intensity**

Replace the intensity line (line 607):

```rust
        let intensity = ext_state.intensity().clamp(0.0, 2.0);
```

with:

```rust
        let intensity = ext_state.intensity().clamp(0.0, 2.0) * transition_factor;
```

- [ ] **Step 9: Handle needs_rebuild from transition**

After the transition processing block, add a check:

```rust
        if needs_rebuild {
            continue;
        }
```

This goes after the transition block and before `canvas.apply_effects`. Note: there is already a `needs_rebuild` check at line 591 — we must NOT duplicate that, but the transition can trigger a rebuild when it switches animations. The existing check at line 591 will catch it on the next iteration.

Actually, we need to handle this differently. The `needs_rebuild` from the transition happens mid-frame, and we want the rebuild to take effect. Add this right after the transition block:

```rust
        // Rebuild if transition triggered a mode change
        if needs_rebuild {
            continue;
        }
```

Wait — we must be careful. The existing `needs_rebuild` check is at line 591, which is BEFORE `set_params` and `update`. If the transition sets `needs_rebuild = true` during step 7, we need to skip this frame. Place this right after the transition block (after step 7 code) and before `anim.set_params`:

```rust
        if needs_rebuild {
            continue;
        }
```

- [ ] **Step 10: Verify build and tests**

Run: `cargo test --quiet 2>&1 && cargo build --quiet 2>&1`
Expected: Compiles and all tests pass.

- [ ] **Step 11: Commit**

```bash
git add src/main.rs
git commit -m "feat: add fade transition effect between animations"
```

---

## Task 9: Add configurable keybindings — config side

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add keybindings field to Config struct**

Add `HashMap` import and the new field to `Config`:

```rust
use std::collections::HashMap;
```

Add field to `Config` struct (after `data_file`):

```rust
    /// Custom keybindings for animation controls
    pub keybindings: Option<HashMap<String, String>>,
```

- [ ] **Step 2: Add keybindings section to default config template**

In `default_config_string()`, append before the closing `"#`:

```toml

# Custom keybindings (any key name: q, n, Right, Left, Esc, Space, Tab, etc.)
# [keybindings]
# next = "Right"
# prev = "Left"
# quit = "q"
# render = "r"
# color = "c"
# status = "h"
```

- [ ] **Step 3: Add keybinding config test**

Add to the test module:

```rust
    #[test]
    fn test_config_parses_keybindings() {
        let toml = r#"
            [keybindings]
            next = "Right"
            quit = "Esc"
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let kb = cfg.keybindings.unwrap();
        assert_eq!(kb.get("next").unwrap(), "Right");
        assert_eq!(kb.get("quit").unwrap(), "Esc");
    }
```

- [ ] **Step 4: Verify tests**

Run: `cargo test --quiet -- config 2>&1`
Expected: All config tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add keybindings section to config schema"
```

---

## Task 10: Add configurable keybindings — main.rs integration

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add parse_key_binding function**

Add this function in `main.rs`, after `parse_color_mode` (after line 770):

```rust
fn parse_key_binding(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let s = s.trim();
    // Handle modifier+key format (e.g. "Ctrl+c")
    if let Some((mods, key)) = s.split_once('+') {
        let key_code = parse_key_code(key.trim())?;
        let modifiers = match mods.trim().to_ascii_lowercase().as_str() {
            "ctrl" => KeyModifiers::CONTROL,
            "alt" => KeyModifiers::ALT,
            "shift" => KeyModifiers::SHIFT,
            _ => return None,
        };
        return Some((key_code, modifiers));
    }
    let key_code = parse_key_code(s)?;
    Some((key_code, KeyModifiers::NONE))
}

fn parse_key_code(s: &str) -> Option<KeyCode> {
    match s {
        "Left" => Some(KeyCode::Left),
        "Right" => Some(KeyCode::Right),
        "Up" => Some(KeyCode::Up),
        "Down" => Some(KeyCode::Down),
        "Esc" => Some(KeyCode::Esc),
        "Enter" => Some(KeyCode::Enter),
        "Space" => Some(KeyCode::Char(' ')),
        "Tab" => Some(KeyCode::Tab),
        s if s.len() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
        _ => None,
    }
}
```

- [ ] **Step 2: Add KeyBindings struct and builder**

Add this struct and builder function near the `parse_key_binding` function:

```rust
struct KeyBindings {
    next: KeyCode,
    prev: KeyCode,
    quit: Vec<KeyCode>,
    render: KeyCode,
    color: KeyCode,
    status: KeyCode,
}

fn build_keybindings(cfg: &config::Config) -> KeyBindings {
    let kb = cfg.keybindings.as_ref();
    let defaults = KeyBindings::defaults();
    KeyBindings {
        next: kb
            .and_then(|m| m.get("next"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| c)
            .unwrap_or(defaults.next),
        prev: kb
            .and_then(|m| m.get("prev"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| c)
            .unwrap_or(defaults.prev),
        quit: {
            let custom = kb
                .and_then(|m| m.get("quit"))
                .and_then(|s| parse_key_binding(s))
                .map(|(c, _)| vec![c]);
            custom.unwrap_or_else(|| vec![KeyCode::Char('q'), KeyCode::Esc])
        },
        render: kb
            .and_then(|m| m.get("render"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| c)
            .unwrap_or(defaults.render),
        color: kb
            .and_then(|m| m.get("color"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| c)
            .unwrap_or(defaults.color),
        status: kb
            .and_then(|m| m.get("status"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| c)
            .unwrap_or(defaults.status),
    }
}

impl KeyBindings {
    fn defaults() -> Self {
        KeyBindings {
            next: KeyCode::Right,
            prev: KeyCode::Left,
            quit: vec![KeyCode::Char('q'), KeyCode::Esc],
            render: KeyCode::Char('r'),
            color: KeyCode::Char('c'),
            status: KeyCode::Char('h'),
        }
    }
}
```

- [ ] **Step 3: Build keybindings and pass to run_loop**

In `main()`, after loading config (`let cfg = config::load_config();`), add:

```rust
    let keybindings = build_keybindings(&cfg);
```

Then add `keybindings: &KeyBindings` parameter to `run_loop` function signature (after `data_file`). Pass `&keybindings` in the call to `run_loop`.

- [ ] **Step 4: Update hotkey match to use KeyBindings**

Replace the hotkey match block (lines 386-455) with:

```rust
                        match code {
                            kc if keybindings.quit.contains(&kc) => {
                                if let (Some(rec), Some(path)) = (recorder.take(), record_path) {
                                    let mut stdout = io::stdout();
                                    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
                                    terminal::disable_raw_mode()?;
                                    rec.save(path)?;
                                    println!("Saved {} frames to {}", rec.frame_count(), path);
                                    terminal::enable_raw_mode()?;
                                    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
                                }
                                return Ok(());
                            }
                            kc if kc == keybindings.next => {
                                anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
                                start_transition(&mut transition, anim_index);
                                cycle_start = Instant::now();
                            }
                            kc if kc == keybindings.prev => {
                                anim_index = if anim_index == 0 {
                                    animations::ANIMATION_NAMES.len() - 1
                                } else {
                                    anim_index - 1
                                };
                                start_transition(&mut transition, anim_index);
                                cycle_start = Instant::now();
                            }
                            kc if kc == keybindings.render => {
                                let idx = RENDER_MODES
                                    .iter()
                                    .position(|&m| m == render_mode)
                                    .unwrap_or(0);
                                render_mode = RENDER_MODES[(idx + 1) % RENDER_MODES.len()];
                                needs_rebuild = true;
                            }
                            kc if kc == keybindings.color => {
                                let idx = COLOR_MODES
                                    .iter()
                                    .position(|&m| m == color_mode)
                                    .unwrap_or(0);
                                color_mode = COLOR_MODES[(idx + 1) % COLOR_MODES.len()];
                                needs_rebuild = true;
                            }
                            kc if kc == keybindings.status => {
                                hide_status = !hide_status;
                                needs_rebuild = true;
                            }
                            _ => {}
                        }
```

Note: The `n` and `p` keys as alternative next/prev bindings are dropped since keybindings are now user-configurable. Users who want them can add `next = "n"` to config. The default is arrow keys only.

Actually, we should keep `n` and `p` as extra defaults alongside arrows. Update the `KeyBindings` struct to support multiple keys:

```rust
struct KeyBindings {
    next: Vec<KeyCode>,
    prev: Vec<KeyCode>,
    quit: Vec<KeyCode>,
    render: Vec<KeyCode>,
    color: Vec<KeyCode>,
    status: Vec<KeyCode>,
}
```

And the match guards become:

```rust
kc if keybindings.next.contains(&kc) => {
```

Update the builder and defaults to use `Vec<KeyCode>`:

```rust
fn build_keybindings(cfg: &config::Config) -> KeyBindings {
    let kb = cfg.keybindings.as_ref();
    let defaults = KeyBindings::defaults();
    KeyBindings {
        next: kb
            .and_then(|m| m.get("next"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.next),
        prev: kb
            .and_then(|m| m.get("prev"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.prev),
        quit: kb
            .and_then(|m| m.get("quit"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.quit),
        render: kb
            .and_then(|m| m.get("render"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.render),
        color: kb
            .and_then(|m| m.get("color"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.color),
        status: kb
            .and_then(|m| m.get("status"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.status),
    }
}

impl KeyBindings {
    fn defaults() -> Self {
        KeyBindings {
            next: vec![KeyCode::Right, KeyCode::Char('n')],
            prev: vec![KeyCode::Left, KeyCode::Char('p')],
            quit: vec![KeyCode::Char('q'), KeyCode::Esc],
            render: vec![KeyCode::Char('r')],
            color: vec![KeyCode::Char('c')],
            status: vec![KeyCode::Char('h')],
        }
    }
}
```

Also update the quit handler check in the chunked write loop (lines 686-688 and 711-713). Replace:

```rust
&& (matches!(code, KeyCode::Char('q') | KeyCode::Esc)
    || (code == KeyCode::Char('c')
        && modifiers.contains(KeyModifiers::CONTROL)))
```

with:

```rust
&& (keybindings.quit.contains(&code)
    || (code == KeyCode::Char('c')
        && modifiers.contains(KeyModifiers::CONTROL)))
```

Since `keybindings` is a reference, we need to make it available in the loop. Since `run_loop` now takes `keybindings: &KeyBindings`, it's in scope.

- [ ] **Step 5: Verify build and tests**

Run: `cargo test --quiet 2>&1 && cargo build --quiet 2>&1`

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: add configurable keybindings from config file"
```

---

## Task 11: Final verification

- [ ] **Step 1: Run full check**

Run: `cargo fmt --check && cargo clippy --quiet 2>&1 && cargo test --quiet 2>&1`

Fix any issues from fmt/clippy.

- [ ] **Step 2: Update ideas.md**

Mark completed items. In `ideas.md`, update the sections:

Under "Dead Code Cleanup", replace the vortex section with:
```
### ~~[cleanup] Register or Remove `vortex` Animation~~ ✅ Done
```

Under "Per-Animation Exposed Parameters", add a note:
```
(Completed for boids, particles, wave, sort, snake, pong)
```

Under "Transition Effects Between Animations", mark done:
```
### ~~[ux] Transition Effects Between Animations~~ ✅ Done
```

Under "Configurable Keybindings", mark done:
```
### ~~[ux] Configurable Keybindings~~ ✅ Done
```

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "docs: mark completed ideas in ideas.md"
```
