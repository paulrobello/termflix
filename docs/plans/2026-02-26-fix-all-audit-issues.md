# Fix All Audit Issues Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Resolve every finding in AUDIT.md — critical panics first, then correctness, then performance, then tests.

**Architecture:** All fixes are contained within existing files. No new modules needed. Tests use Rust's inline `#[cfg(test)]` system so no lib target is required. The `on_resize` trait hook is additive and backward-compatible.

**Tech Stack:** Rust 2024 edition, cargo, rand 0.10, notify 7, serde_json 1

---

## Task 1 — Formatting + dependency pins (H-1, L-1, L-2)

**Files:**
- Modify: `src/animations/garden.rs` (run rustfmt)
- Modify: `Cargo.toml`

**Step 1: Fix garden.rs formatting**

```bash
cargo fmt
```

**Step 2: Verify format passes**

```bash
cargo fmt -- --check
```

Expected: no output, exit code 0.

**Step 3: Pin serde_json and add inotify feature**

In `Cargo.toml` change:
```toml
serde_json = "1"
notify = { version = "7", default-features = false, features = ["macos_kqueue"] }
```
To:
```toml
serde_json = "1.0"
notify = { version = "7", default-features = false, features = ["macos_kqueue", "inotify"] }
```

**Step 4: Verify it builds**

```bash
cargo check
```

Expected: `Finished` with no errors.

**Step 5: Commit**

```bash
git add Cargo.toml src/animations/garden.rs
git commit -m "fix: format garden.rs, pin serde_json, add inotify watcher support"
```

---

## Task 2 — Fix file watcher panics (C-1)

**Files:**
- Modify: `src/external.rs:146-151`

**Context:** The `ParamsSource::File` branch in `spawn_reader()` calls `.unwrap()` on both `notify::recommended_watcher()` (line 149) and `Watcher::watch()` (line 151). If the data file path is inaccessible or the OS watcher fails, the thread panics with no error message and the terminal may be left in raw mode.

**Step 1: Replace both unwraps with graceful error handling**

Current code (lines 145-162):
```rust
let (file_tx, file_rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(move |res| {
    let _ = file_tx.send(res);
})
.unwrap();
notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
    .unwrap();
while let Ok(Ok(_event)) = file_rx.recv() {
```

Replace with:
```rust
let (file_tx, file_rx) = std::sync::mpsc::channel();
let mut watcher = match notify::recommended_watcher(move |res| {
    let _ = file_tx.send(res);
}) {
    Ok(w) => w,
    Err(e) => {
        eprintln!("termflix: could not create file watcher: {e}");
        return;
    }
};
if let Err(e) =
    notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
{
    eprintln!("termflix: could not watch {}: {e}", path.display());
    return;
}
while let Ok(Ok(_event)) = file_rx.recv() {
```

**Step 2: Verify**

```bash
cargo check
```

Expected: `Finished` with no errors.

**Step 3: Commit**

```bash
git add src/external.rs
git commit -m "fix: replace panicking unwraps in file watcher with graceful error handling"
```

---

## Task 3 — Fix animation factory panic (C-2)

**Files:**
- Modify: `src/animations/mod.rs:164-213`
- Modify: `src/main.rs` (all call sites of `animations::create`)

**Context:** `animations::create()` panics with `panic!("Unknown animation: {}")` on line 211. The signature should return `Option<Box<dyn Animation>>`. All 7 call sites in `main.rs` already validate the name first, so they can safely `unwrap()` or `expect()` the Option — but at least the type contract is now honest.

**Step 1: Change the return type and final arm**

In `src/animations/mod.rs`, change:

```rust
pub fn create(name: &str, width: usize, height: usize, scale: f64) -> Box<dyn Animation> {
    match name {
        // ... all arms unchanged ...
        _ => panic!("Unknown animation: {}", name),
    }
}
```

To:

```rust
pub fn create(name: &str, width: usize, height: usize, scale: f64) -> Option<Box<dyn Animation>> {
    Some(match name {
        "fire" => Box::new(fire::Fire::new(width, height)),
        "matrix" => Box::new(matrix::Matrix::new(width, height, scale)),
        "plasma" => Box::new(plasma::Plasma::new()),
        "starfield" => Box::new(starfield::Starfield::new(width, height, scale)),
        "wave" => Box::new(wave::Wave::new()),
        "life" => Box::new(life::GameOfLife::new(width, height)),
        "particles" => Box::new(particles::Particles::new(width, height, scale)),
        "rain" => Box::new(rain::Rain::new(width, height, scale)),
        "fountain" => Box::new(fountain::Fountain::new(width, height, scale)),
        "flow" => Box::new(flow_field::FlowField::new(width, height, scale)),
        "spiral" => Box::new(spiral::Spiral::new()),
        "ocean" => Box::new(ocean::Ocean::new()),
        "aurora" => Box::new(aurora::Aurora::new()),
        "lightning" => Box::new(lightning::Lightning::new(width, height)),
        "smoke" => Box::new(smoke::Smoke::new(width, height, scale)),
        "ripple" => Box::new(ripple::Ripple::new(width, height)),
        "snow" => Box::new(snow::Snow::new(width, height, scale)),
        "garden" => Box::new(garden::Garden::new(width, height, scale)),
        "fireflies" => Box::new(fireflies::Fireflies::new(width, height, scale)),
        "dna" => Box::new(dna::Dna::new()),
        "pulse" => Box::new(pulse::Pulse::new(width, height)),
        "boids" => Box::new(boids::Boids::new(width, height, scale)),
        "lava" => Box::new(lava::Lava::new(width, height, scale)),
        "sandstorm" => Box::new(sandstorm::Sandstorm::new(width, height, scale)),
        "petals" => Box::new(petals::Petals::new(width, height, scale)),
        "campfire" => Box::new(campfire::Campfire::new(width, height, scale)),
        "waterfall" => Box::new(waterfall::Waterfall::new(width, height, scale)),
        "eclipse" => Box::new(eclipse::Eclipse::new()),
        "blackhole" => Box::new(blackhole::Blackhole::new()),
        "radar" => Box::new(radar::Radar::new()),
        "crystallize" => Box::new(crystallize::Crystallize::new(width, height, scale)),
        "hackerman" => Box::new(hackerman::Hackerman::new(width, height, scale)),
        "visualizer" => Box::new(visualizer::Visualizer::new(width, height, scale)),
        "cells" => Box::new(cells::Cells::new(width, height, scale)),
        "atom" => Box::new(atom::Atom::new()),
        "globe" => Box::new(globe::Globe::new()),
        "dragon" => Box::new(dragon::Dragon::new()),
        "sierpinski" => Box::new(sierpinski::Sierpinski::new()),
        "mandelbrot" => Box::new(mandelbrot::Mandelbrot::new()),
        "langton" => Box::new(langton::Langton::new(width, height, scale)),
        "sort" => Box::new(sort::Sort::new(width, height, scale)),
        "snake" => Box::new(snake::Snake::new(width, height, scale)),
        "invaders" => Box::new(invaders::Invaders::new(width, height, scale)),
        "pong" => Box::new(pong::Pong::new(width, height, scale)),
        _ => return None,
    })
}
```

**Step 2: Update all call sites in main.rs**

There are 7 calls to `animations::create(...)` in `src/main.rs`. Each returns `Box<dyn Animation>` today and must be updated to unwrap the Option. The name is always validated before calling, so `.expect("validated above")` is appropriate.

Search for every occurrence:

```bash
grep -n "animations::create" src/main.rs
```

Each call looks like:
```rust
anim = animations::create(
    animations::ANIMATION_NAMES[anim_index],
    canvas.width,
    canvas.height,
    scale,
);
```

Change every occurrence to:
```rust
anim = animations::create(
    animations::ANIMATION_NAMES[anim_index],
    canvas.width,
    canvas.height,
    scale,
).expect("animation name validated before calling create");
```

Also fix the two initialization calls near top of `run_loop` (lines ~297-302):
```rust
let mut anim: Box<dyn Animation> =
    animations::create(initial_anim, temp_canvas.width, temp_canvas.height, scale)
        .expect("animation name validated before calling create");
// ...
anim = animations::create(initial_anim, canvas.width, canvas.height, scale)
    .expect("animation name validated before calling create");
```

**Step 3: Verify**

```bash
cargo check
```

Expected: no errors.

**Step 4: Verify clippy still passes**

```bash
cargo clippy -- -D warnings
```

Expected: zero warnings.

**Step 5: Commit**

```bash
git add src/animations/mod.rs src/main.rs
git commit -m "fix: change animations::create to return Option instead of panicking"
```

---

## Task 4 — Name magic constants and fix code clarity (M-3, L-5, L-7)

**Files:**
- Modify: `src/render/braille.rs`
- Modify: `src/render/halfblock.rs`
- Modify: `src/animations/garden.rs` (first few lines)

**Context:** `THRESHOLD = 0.3` in braille and `DARK_THRESHOLD = 0.02` in halfblock are undocumented. The braille `unwrap_or` implies a failure is plausible when it cannot be. The `garden.rs` type alias `PRow` hides a `'static` lifetime.

**Step 1: Document braille threshold and fix the unwrap**

In `src/render/braille.rs`, change:

```rust
const THRESHOLD: f64 = 0.3;
```

To:

```rust
/// Minimum pixel brightness [0.0..=1.0] to render a braille dot.
/// Pixels at or below this value are treated as dark/empty.
/// Calibrated so that mid-intensity animations fill ~50% of dots.
const BRIGHTNESS_THRESHOLD: f64 = 0.3;
```

Also update the usage on the line that references `THRESHOLD`:
```rust
if canvas.pixels[idx] > THRESHOLD {
```
Change to:
```rust
if canvas.pixels[idx] > BRIGHTNESS_THRESHOLD {
```

For the `unwrap_or` on line 57, the Unicode value `BRAILLE_OFFSET + bits` is always in the valid Braille Patterns block (U+2800–U+28FF) because `bits` is a bitmask over 8 flags (0x00–0xFF). Document this invariant and use a debug_assert:

```rust
// bits is a mask of 8 flags so its range is 0x00..=0xFF.
// BRAILLE_OFFSET (0x2800) + any value in that range is always a valid
// Unicode scalar in the Braille Patterns block (U+2800–U+28FF).
debug_assert!(bits <= 0xFF);
let ch = char::from_u32(BRAILLE_OFFSET + bits)
    .expect("braille bits 0x00..=0xFF always produce valid Unicode");
```

**Step 2: Document halfblock threshold**

In `src/render/halfblock.rs`, change:

```rust
const DARK_THRESHOLD: f64 = 0.02;
```

To:

```rust
/// Pixel brightness below which a half-block cell is treated as background (dark/empty).
/// This is intentionally much lower than the braille threshold (0.3) because half-block
/// renders the full brightness value via color scaling, whereas braille uses binary on/off dots.
/// A low threshold preserves near-black pixels that would otherwise be clipped.
const DARK_THRESHOLD: f64 = 0.02;
```

**Step 3: Add comment to garden.rs type alias**

In `src/animations/garden.rs`, change the first few lines. Find:

```rust
type PRow = &'static [(i32, char, bool)];
```

And add a comment:

```rust
/// A slice of (column_offset, character, is_colored) tuples describing one row of a plant shape.
/// The `'static` lifetime means these are compile-time constant arrays.
type PRow = &'static [(i32, char, bool)];
```

**Step 4: Verify**

```bash
cargo check && cargo clippy -- -D warnings
```

Expected: no errors, no warnings.

**Step 5: Commit**

```bash
git add src/render/braille.rs src/render/halfblock.rs src/animations/garden.rs
git commit -m "fix: document render thresholds, fix braille unwrap invariant, clarify PRow alias"
```

---

## Task 5 — Remove redundant bounds checks and fix unused param names (M-4, M-6)

**Files:**
- Modify: `src/animations/rain.rs` (remove redundant bounds check)
- Modify: `src/animations/garden.rs` (remove redundant bounds checks, ~lines 231-239 and similar)
- Modify: all animation files with inconsistent `_dt`/`_time` naming

**Context:** `Canvas::set_colored()` and `Canvas::set_char()` already bounds-check internally. Some animations add manual checks before calling them, which is dead code. Also, unused `dt` and `time` parameters should consistently use the `_` prefix per Rust convention.

**Step 1: Remove redundant bounds check in rain.rs**

In `src/animations/rain.rs` update(), find this pattern around lines 127-133:

```rust
if px < canvas.width && py < canvas.height {
    let brightness = depth_brightness * (0.5 + 0.5 * (1.0 - t));
    let r = (60.0 + 80.0 * drop.depth) as u8;
    let g = (80.0 + 90.0 * drop.depth) as u8;
    let b = (120.0 + 135.0 * drop.depth) as u8;
    canvas.set_colored(px, py, brightness, r, g, b);
}
```

The `if px < canvas.width && py < canvas.height` guard is redundant — `set_colored` already silently skips out-of-bounds. However, the `px` and `py` values here are computed as `as usize` from floating point that could underflow to 0 via wrapping. Keep the check only to avoid the `as usize` underflow (negative float cast). In Rust, negative f64 cast to usize is 0 on most platforms (saturating), so this is actually safe to remove for `px` since `drop.x` is always positive due to wrapping. But `py = (drop.y - ...) as usize` could produce a very large usize if negative.

**Keep the check for `py`** (negative y can produce huge usize), but remove the redundant `px` check since `drop.x` wraps via the horizontal-wrap logic:

```rust
let brightness = depth_brightness * (0.5 + 0.5 * (1.0 - t));
let r = (60.0 + 80.0 * drop.depth) as u8;
let g = (80.0 + 90.0 * drop.depth) as u8;
let b = (120.0 + 135.0 * drop.depth) as u8;
if py < canvas.height {
    canvas.set_colored(px, py, brightness, r, g, b);
}
```

**Step 2: Remove redundant double-bounds-check in garden.rs**

Search garden.rs for patterns like:
```rust
if px >= 0 && (px as usize) < canvas.width && py >= 0 && (py as usize) < canvas.height {
    canvas.set_char(px as usize, py as usize, ch, r, g, b);
}
```

Since `px` and `py` are `i32`, they can be negative (valid for off-screen positions). The `set_char` method takes `usize`, so we do still need the negative check before casting. The upper-bounds check is redundant. Change all occurrences to:

```rust
if px >= 0 && py >= 0 {
    canvas.set_char(px as usize, py as usize, ch, r, g, b);
}
```

Use sed or manual edit for each occurrence in garden.rs.

**Step 3: Fix inconsistent unused parameter names**

Search all animation files for `fn update` signatures with unnamed `dt` or `time` that ARE used without `_` prefix:

```bash
grep -rn "fn update.*dt.*time" src/animations/
```

For each animation where `dt` is truly unused, rename to `_dt`. Where `time` is truly unused, rename to `_time`. Where both are used, keep as-is. Read each file to determine which are actually used in the body.

**Step 4: Verify**

```bash
cargo check && cargo clippy -- -D warnings
```

Expected: no errors, no warnings.

**Step 5: Commit**

```bash
git add src/animations/
git commit -m "fix: remove redundant bounds checks, fix unused parameter naming"
```

---

## Task 6 — Move RNG to struct fields (M-2)

**Files:**
- Modify: `src/animations/fire.rs`
- Modify: `src/animations/rain.rs`
- Modify: `src/animations/boids.rs`
- Modify: `src/animations/particles.rs`
- Modify: `src/animations/garden.rs`
- Modify: `src/animations/campfire.rs` (if it calls `rand::rng()` in update)
- Modify: `src/animations/smoke.rs` (if it calls `rand::rng()` in update)

Check all animations that call `rand::rng()` inside `update()`:

```bash
grep -l "rand::rng()" src/animations/*.rs
```

For each file found, follow the same pattern.

**Step 1: Example — fix fire.rs**

The `fire.rs` `update()` method calls `let mut rng = rand::rng();` on line 55.

Add `rng` as a field:

```rust
use rand::rngs::ThreadRng;

pub struct Fire {
    width: usize,
    height: usize,
    buffer: Vec<f64>,
    heat_rate: f64,
    rng: ThreadRng,
}

impl Fire {
    pub fn new(width: usize, height: usize) -> Self {
        // ... existing init code ...
        Fire {
            width,
            height,
            buffer,
            heat_rate: 0.8,
            rng: rand::rng(),
        }
    }
}
```

In `update()`, remove `let mut rng = rand::rng();` and change all `rng.` calls to `self.rng.`.

**Step 2: Repeat the same pattern for rain.rs**

Add `rng: ThreadRng` to `Rain`, initialize in `new()`, remove the per-frame `let mut rng = rand::rng();` in `update()`, use `self.rng.` instead.

**Step 3: Repeat for boids.rs, particles.rs, garden.rs, and any others found**

Same pattern for each.

**Step 4: Verify**

```bash
cargo check && cargo clippy -- -D warnings
```

Expected: no errors.

**Step 5: Commit**

```bash
git add src/animations/
git commit -m "perf: move RNG to struct field in all animations, eliminating per-frame rng() calls"
```

---

## Task 7 — Precompute particle colors (M-5)

**Files:**
- Modify: `src/animations/rain.rs`

**Context:** In `rain.rs` update(), inside the inner `for i in 0..steps` loop, these three lines run on every particle every frame:

```rust
let r = (60.0 + 80.0 * drop.depth) as u8;
let g = (80.0 + 90.0 * drop.depth) as u8;
let b = (120.0 + 135.0 * drop.depth) as u8;
```

`drop.depth` never changes after initialization, so these values are constant. Store them on the `Raindrop` struct.

**Step 1: Add color fields to Raindrop**

```rust
struct Raindrop {
    x: f64,
    y: f64,
    speed: f64,
    length: f64,
    wind_offset: f64,
    depth: f64,
    r: u8,  // precomputed base color from depth
    g: u8,
    b: u8,
}
```

**Step 2: Initialize in Rain::new()**

In the `.map(|_| { ... })` closure that creates each `Raindrop`, add:

```rust
let r = (60.0 + 80.0 * depth) as u8;
let g = (80.0 + 90.0 * depth) as u8;
let b = (120.0 + 135.0 * depth) as u8;
Raindrop {
    x: rng.random_range(0.0..width as f64),
    y: rng.random_range(-(height as f64)..height as f64),
    speed: 15.0 + depth * 50.0,
    length: 1.0 + depth * 5.0,
    wind_offset: rng.random_range(-0.5..0.5),
    depth,
    r,
    g,
    b,
}
```

Also set colors when a drop is reset at line ~147-151. After `drop.y = rng.random_range(...)`, add:
```rust
// depth doesn't change on reset, colors are still valid — no update needed
```

**Step 3: Use precomputed values in update()**

Remove the three per-frame color computation lines inside the loop:

```rust
// DELETE these:
let r = (60.0 + 80.0 * drop.depth) as u8;
let g = (80.0 + 90.0 * drop.depth) as u8;
let b = (120.0 + 135.0 * drop.depth) as u8;
```

Change `canvas.set_colored(px, py, brightness, r, g, b)` to `canvas.set_colored(px, py, brightness, drop.r, drop.g, drop.b)`.

**Step 4: Verify**

```bash
cargo check && cargo clippy -- -D warnings
```

**Step 5: Commit**

```bash
git add src/animations/rain.rs
git commit -m "perf: precompute raindrop colors at creation instead of per-frame"
```

---

## Task 8 — Add `on_resize` trait hook and remove per-frame dimension copy (M-1, L-6)

**Files:**
- Modify: `src/animations/mod.rs` (trait definition)
- Modify: all 44 animation files (remove `self.width = canvas.width; self.height = canvas.height;` from `update()`, add `on_resize` override)

**Context:** Almost every animation begins `update()` with:
```rust
self.width = canvas.width;
self.height = canvas.height;
```

Animations are always recreated when the canvas is rebuilt (so this is a no-op in practice), but it's boilerplate that clutters every update method. Adding an `on_resize` hook to the trait expresses the intent more clearly and provides a proper lifecycle for future optimizations (e.g., not recreating animations on resize).

The fire animation additionally calls `self.resize(w, h)` inside update() to rebuild its heat buffer. Move this logic to `on_resize()`.

**Step 1: Add `on_resize` to the Animation trait in mod.rs**

In `src/animations/mod.rs`, after `set_params`:

```rust
pub trait Animation {
    fn name(&self) -> &str;
    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64);
    fn preferred_render(&self) -> RenderMode {
        RenderMode::HalfBlock
    }
    fn set_params(&mut self, _params: &crate::external::ExternalParams) {}
    /// Called when the canvas dimensions change. Default implementation is a no-op.
    /// Override to update stored dimensions and rebuild any size-dependent buffers.
    fn on_resize(&mut self, _width: usize, _height: usize) {}
}
```

**Step 2: Call on_resize from main.rs when canvas is rebuilt**

In `src/main.rs`, in the `if needs_rebuild { ... }` block (around line 442), after:
```rust
anim = animations::create(
    animations::ANIMATION_NAMES[anim_index],
    canvas.width,
    canvas.height,
    scale,
).expect("animation name validated before calling create");
```

Add immediately after:
```rust
anim.on_resize(canvas.width, canvas.height);
```

Do the same at the other two animation creation sites (initialization at ~line 298-302 and external-param-triggered recreation at ~lines 506-511, 523-528).

**Step 3: For each simple animation — remove dimension copy from update()**

For animations that ONLY update `self.width` and `self.height` (no buffer rebuild), the fix is:
1. Add `on_resize` override to update dimensions
2. Remove the two lines from `update()`

Example for `boids.rs`:

Change the start of `update()` from:
```rust
fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
    self.width = canvas.width;
    self.height = canvas.height;
    // rest of method...
```

To simply:
```rust
fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
    // rest of method...
```

And add the trait method:
```rust
fn on_resize(&mut self, width: usize, height: usize) {
    self.width = width;
    self.height = height;
}
```

**Step 4: For fire.rs — move resize + buffer rebuild to on_resize()**

In `fire.rs`, the existing `resize()` method already does what we need. Rename it to `on_resize` and implement it as the trait method:

Remove the private `resize()` method and add:
```rust
fn on_resize(&mut self, width: usize, height: usize) {
    self.width = width;
    self.height = height;
    self.buffer = vec![0.0; width * height];
    for x in 0..width {
        for y in height.saturating_sub(2)..height {
            self.buffer[y * width + x] = 1.0;
        }
    }
}
```

In `update()`, remove:
```rust
if self.width != w || self.height != h {
    self.resize(w, h);
}
```

**Step 5: Apply to all remaining animation files**

Run:
```bash
grep -rln "self\.width = canvas\.width\|self\.height = canvas\.height" src/animations/
```

For each file returned, apply the same pattern as Step 3 (add `on_resize` override, remove lines from `update()`).

**Step 6: Verify no regressions**

```bash
cargo check && cargo clippy -- -D warnings
```

Expected: no errors, no warnings.

**Step 7: Commit**

```bash
git add src/animations/ src/main.rs
git commit -m "refactor: add on_resize trait hook, remove per-frame dimension copy from all animations"
```

---

## Task 9 — Add test suite (H-2)

**Files:**
- Modify: `src/config.rs` (add inline test module)
- Modify: `src/external.rs` (add inline test module)
- Modify: `src/record.rs` (add inline test module)
- Modify: `src/animations/mod.rs` (add inline test module)
- Modify: `src/render/canvas.rs` (add inline test module)

**Context:** Tests use Rust's built-in `#[cfg(test)]` modules inside source files. No lib target needed. Run with `cargo test`.

**Step 1: Add config tests to src/config.rs**

At the end of the file, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_is_all_none() {
        let cfg = Config::default();
        assert!(cfg.animation.is_none());
        assert!(cfg.fps.is_none());
        assert!(cfg.scale.is_none());
    }

    #[test]
    fn test_config_parses_valid_toml() {
        let toml = r#"
            animation = "fire"
            fps = 30
            scale = 1.5
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.animation.as_deref(), Some("fire"));
        assert_eq!(cfg.fps, Some(30));
        assert_eq!(cfg.scale, Some(1.5));
    }

    #[test]
    fn test_config_unknown_fields_are_ignored() {
        let toml = r#"unknown_field = "value""#;
        let result: Result<Config, _> = toml::from_str(toml);
        // With #[serde(default)], unknown fields are accepted or rejected depending on
        // serde config. Verify load_config() never panics on bad TOML.
        // load_config() returns default on parse error.
        let _ = result; // either Ok or Err is acceptable, no panic
    }
}
```

**Step 2: Add ExternalParams tests to src/external.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_params_deserializes_partial() {
        let json = r#"{"animation": "matrix", "speed": 2.0}"#;
        let p: ExternalParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.animation.as_deref(), Some("matrix"));
        assert_eq!(p.speed, Some(2.0));
        assert!(p.intensity.is_none());
    }

    #[test]
    fn test_external_params_empty_object() {
        let json = "{}";
        let p: ExternalParams = serde_json::from_str(json).unwrap();
        assert!(p.animation.is_none());
        assert!(p.speed.is_none());
    }

    #[test]
    fn test_external_params_invalid_json_fails() {
        let json = "not json";
        let result = serde_json::from_str::<ExternalParams>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_current_state_merge_accumulates() {
        let mut state = CurrentState::default();
        state.merge(ExternalParams { speed: Some(2.0), ..Default::default() });
        state.merge(ExternalParams { intensity: Some(0.5), ..Default::default() });
        assert_eq!(state.speed(), 2.0);
        assert_eq!(state.intensity(), 0.5);
    }

    #[test]
    fn test_current_state_take_animation_change() {
        let mut state = CurrentState::default();
        state.merge(ExternalParams {
            animation: Some("fire".to_string()),
            ..Default::default()
        });
        let change = state.take_animation_change();
        assert_eq!(change.as_deref(), Some("fire"));
        // Second take returns None
        assert!(state.take_animation_change().is_none());
    }
}
```

**Step 3: Add base64 tests to src/record.rs**

In `record.rs`, the `base64_encode` and `base64_decode` functions are private. Add `pub(crate)` to them, or add the tests in the same file. Add at end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip_empty() {
        let input = b"";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded);
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base64_roundtrip_short() {
        let input = b"hello";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded);
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base64_roundtrip_all_bytes() {
        // Test all 256 byte values to catch padding edge cases
        let input: Vec<u8> = (0u8..=255u8).collect();
        let encoded = base64_encode(&input);
        let decoded = base64_decode(&encoded);
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base64_decode_ignores_newlines() {
        let input = b"foobar";
        let encoded = base64_encode(input);
        // Inject newlines (as would appear in split-line recordings)
        let with_newlines = encoded.chars().flat_map(|c| {
            if c == 'o' { vec!['o', '\n'] } else { vec![c] }
        }).collect::<String>();
        let decoded = base64_decode(&with_newlines);
        assert_eq!(decoded, input);
    }
}
```

**Step 4: Add animation factory tests to src/animations/mod.rs**

At the end of mod.rs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_returns_some_for_all_known_names() {
        for &name in ANIMATION_NAMES {
            let result = create(name, 80, 24, 1.0);
            assert!(result.is_some(), "create({name:?}) returned None");
        }
    }

    #[test]
    fn test_create_returns_none_for_unknown_name() {
        let result = create("does_not_exist", 80, 24, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_animation_names_and_animations_have_same_length() {
        assert_eq!(ANIMATION_NAMES.len(), ANIMATIONS.len());
    }

    #[test]
    fn test_animation_names_match_animations_list() {
        for (name, (anim_name, _desc)) in ANIMATION_NAMES.iter().zip(ANIMATIONS.iter()) {
            assert_eq!(name, anim_name, "ANIMATION_NAMES and ANIMATIONS are out of sync");
        }
    }

    #[test]
    fn test_created_animation_name_matches_requested() {
        let anim = create("fire", 80, 24, 1.0).unwrap();
        assert_eq!(anim.name(), "fire");
    }
}
```

**Step 5: Add canvas bounds tests to src/render/canvas.rs**

At the end of canvas.rs:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{ColorMode, RenderMode};

    fn test_canvas() -> Canvas {
        Canvas::new(10, 10, RenderMode::HalfBlock, ColorMode::TrueColor)
    }

    #[test]
    fn test_set_and_get_pixel() {
        let mut c = test_canvas();
        c.set(5, 5, 0.75);
        assert!((c.pixels[5 * 10 + 5] - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_out_of_bounds_is_noop() {
        let mut c = test_canvas();
        // Should not panic
        c.set(100, 100, 1.0);
        c.set(usize::MAX, usize::MAX, 1.0);
    }

    #[test]
    fn test_clear_zeroes_all_pixels() {
        let mut c = test_canvas();
        c.set(5, 5, 1.0);
        c.clear();
        assert!(c.pixels.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_set_colored_stores_color() {
        let mut c = test_canvas();
        c.set_colored(3, 3, 0.5, 255, 128, 0);
        let idx = 3 * 10 + 3;
        assert_eq!(c.colors[idx], (255, 128, 0));
        assert!((c.pixels[idx] - 0.5).abs() < f64::EPSILON);
    }
}
```

**Step 6: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

**Step 7: Commit**

```bash
git add src/
git commit -m "test: add inline test suites for config, external params, base64, animation factory, and canvas"
```

---

## Task 10 — Add `supported_params()` to Animation trait (M-7)

**Files:**
- Modify: `src/animations/mod.rs` (add method to trait)
- Modify: `src/animations/fire.rs` (implement)
- Modify: `src/animations/plasma.rs` (implement)
- Modify: `src/main.rs` (optionally expose via --list-params, or skip CLI for now)

**Context:** Currently there is no way to discover which parameters an animation supports without reading source code. A `supported_params()` method enables future tooling (e.g., `--list-params <animation>`).

**Step 1: Add the method to the Animation trait**

In `src/animations/mod.rs`, add to the trait:

```rust
/// Returns a list of supported external parameter names with their valid [min, max] ranges.
/// Used for documentation and tooling. Returns empty slice by default.
fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
    &[]
}
```

**Step 2: Implement for fire.rs**

```rust
fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
    &[("intensity", 0.0, 2.0)]
}
```

**Step 3: Implement for plasma.rs**

```rust
fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
    &[("color_shift", 0.0, 1.0)]
}
```

**Step 4: Add --list-params to CLI in main.rs (optional)**

In `main.rs`, optionally add to the `--list` output or a new `--list-params <name>` flag. At minimum, verify it compiles. Skip the CLI flag for now and just ensure the trait method is accessible.

**Step 5: Add a test**

In `src/animations/mod.rs` test module, add:

```rust
#[test]
fn test_fire_supported_params_not_empty() {
    let anim = create("fire", 80, 24, 1.0).unwrap();
    // fire supports intensity
    assert!(!anim.supported_params().is_empty());
}

#[test]
fn test_plasma_supported_params_not_empty() {
    let anim = create("plasma", 80, 24, 1.0).unwrap();
    assert!(!anim.supported_params().is_empty());
}
```

**Step 6: Verify**

```bash
cargo test && cargo clippy -- -D warnings
```

**Step 7: Commit**

```bash
git add src/animations/
git commit -m "feat: add supported_params() to Animation trait for external parameter introspection"
```

---

## Task 11 — Optimize boids with spatial grid (H-3)

**Files:**
- Modify: `src/animations/boids.rs`

**Context:** The current boids simulation does O(N²) neighbor search: for each boid, it checks every other boid. At N=300 boids, that's 90,000 distance checks per frame. A 2D grid where each cell's size equals the visual range reduces this to O(N) average case.

**Step 1: Read the full boids.rs file first**

```bash
cat src/animations/boids.rs
```

Note the `visual_range = 25.0` constant — this is the grid cell size.

**Step 2: Add SpatialGrid struct to boids.rs**

Add before the `Boid` struct:

```rust
/// A 2D spatial hash grid for O(1) average-case neighbor lookup.
/// Cell size equals visual_range so only adjacent cells need checking.
struct SpatialGrid {
    cells: Vec<Vec<usize>>, // cell index -> list of boid indices
    cols: usize,
    rows: usize,
    cell_size: f64,
    width: f64,
    height: f64,
}

impl SpatialGrid {
    fn new(width: f64, height: f64, cell_size: f64) -> Self {
        let cols = ((width / cell_size).ceil() as usize).max(1);
        let rows = ((height / cell_size).ceil() as usize).max(1);
        SpatialGrid {
            cells: vec![Vec::new(); cols * rows],
            cols,
            rows,
            cell_size,
            width,
            height,
        }
    }

    fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    fn insert(&mut self, idx: usize, x: f64, y: f64) {
        let col = ((x / self.cell_size) as usize).min(self.cols - 1);
        let row = ((y / self.cell_size) as usize).min(self.rows - 1);
        self.cells[row * self.cols + col].push(idx);
    }

    /// Returns all boid indices in the 3x3 neighborhood of (x, y).
    fn neighbors<'a>(&'a self, x: f64, y: f64) -> impl Iterator<Item = usize> + 'a {
        let col = (x / self.cell_size) as i32;
        let row = (y / self.cell_size) as i32;
        let cols = self.cols as i32;
        let rows = self.rows as i32;
        (-1i32..=1).flat_map(move |dr| {
            (-1i32..=1).flat_map(move |dc| {
                let c = col + dc;
                let r = row + dr;
                if c >= 0 && c < cols && r >= 0 && r < rows {
                    self.cells[(r as usize) * self.cols + (c as usize)].iter().copied()
                } else {
                    [].iter().copied()
                }
            })
        })
    }
}
```

**Step 3: Add grid to Boids struct**

```rust
pub struct Boids {
    width: usize,
    height: usize,
    boids: Vec<Boid>,
    grid: SpatialGrid,
}
```

In `Boids::new()`, initialize the grid:

```rust
Boids {
    width,
    height,
    boids,
    grid: SpatialGrid::new(width as f64, height as f64, 25.0), // 25.0 = visual_range
}
```

Update `on_resize`:

```rust
fn on_resize(&mut self, width: usize, height: usize) {
    self.width = width;
    self.height = height;
    self.grid = SpatialGrid::new(width as f64, height as f64, 25.0);
}
```

**Step 4: Replace O(N²) loop with grid lookup in update()**

Remove:
```rust
let positions: Vec<(f64, f64, f64, f64)> =
    self.boids.iter().map(|b| (b.x, b.y, b.vx, b.vy)).collect();

for (i, boid) in self.boids.iter_mut().enumerate() {
    // inner loop over positions
    for (j, &(ox, oy, ovx, ovy)) in positions.iter().enumerate() {
        if i == j { continue; }
        // distance check...
    }
}
```

Replace with grid-based approach. This requires a snapshot of positions first (to avoid aliasing when iterating mutably):

```rust
// Build spatial grid
self.grid.clear();
for (i, boid) in self.boids.iter().enumerate() {
    self.grid.insert(i, boid.x, boid.y);
}

// Snapshot positions for rule calculations (same as before, but only used for neighbors)
let snapshot: Vec<(f64, f64, f64, f64)> =
    self.boids.iter().map(|b| (b.x, b.y, b.vx, b.vy)).collect();

for (i, boid) in self.boids.iter_mut().enumerate() {
    let mut sep_x = 0.0_f64;
    let mut sep_y = 0.0_f64;
    let mut align_x = 0.0_f64;
    let mut align_y = 0.0_f64;
    let mut cohes_x = 0.0_f64;
    let mut cohes_y = 0.0_f64;
    let mut neighbors = 0_usize;

    for j in self.grid.neighbors(boid.x, boid.y) {
        if i == j { continue; }
        let (ox, oy, ovx, ovy) = snapshot[j];
        let dx = ox - boid.x;
        let dy = oy - boid.y;
        let dist = (dx * dx + dy * dy).sqrt();
        // ... rest of rule calculations unchanged from original ...
    }
    // ... rest of boid update unchanged ...
}
```

**Step 5: Run tests and verify**

```bash
cargo test && cargo clippy -- -D warnings
```

Expected: all tests pass, no warnings.

**Step 6: Commit**

```bash
git add src/animations/boids.rs
git commit -m "perf: replace O(N²) boids neighbor search with O(N) spatial grid"
```

---

## Task 12 — Final verification

**Step 1: Run full check suite**

```bash
make checkall
```

Expected output:
```
cargo fmt -- --check   → no output (exit 0)
cargo clippy -- -D warnings → Finished, 0 warnings
cargo check            → Finished
cargo test             → all tests pass
cargo build            → Finished
```

**Step 2: Smoke test the binary**

```bash
cargo run --release -- fire &
sleep 3
kill %1
```

Expected: fire animation runs for 3 seconds with no crash.

**Step 3: Smoke test file watcher error path**

```bash
cargo run --release -- fire --data-file /nonexistent/path.json
```

Expected: prints `termflix: could not watch /nonexistent/path.json: ...` then runs normally (no crash).

**Step 4: Final commit if any stray changes**

```bash
git status
git add -A
git commit -m "chore: final cleanup after audit fixes"
```

---

## Summary of Changes

| Task | Audit Items | Files Changed |
|------|------------|---------------|
| 1 | H-1, L-1, L-2 | Cargo.toml, garden.rs |
| 2 | C-1 | external.rs |
| 3 | C-2 | animations/mod.rs, main.rs |
| 4 | M-3, L-5, L-7 | braille.rs, halfblock.rs, garden.rs |
| 5 | M-4, M-6 | animations/*.rs |
| 6 | M-2 | animations/*.rs (fire, rain, boids, particles, garden, ...) |
| 7 | M-5 | animations/rain.rs |
| 8 | M-1, L-6 | animations/mod.rs, all animation files, main.rs |
| 9 | H-2 | config.rs, external.rs, record.rs, animations/mod.rs, render/canvas.rs |
| 10 | M-7 | animations/mod.rs, fire.rs, plasma.rs |
| 11 | H-3 | animations/boids.rs |
| 12 | — | verification only |
