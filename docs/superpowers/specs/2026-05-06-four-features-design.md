# termflix: Four Features Design

**Date:** 2026-05-06

## Overview

Four independent features from ideas.md, each touching different files with no cross-dependencies:

1. Post-Processing Effects (bloom, vignette, scanlines)
2. GIF Export from .asciianim recordings
3. Macro-Based Animation Registration
4. Unified Particle System

---

## Feature 1: Post-Processing Effects

### Goal

Add optional post-processing passes to the canvas: bloom/glow, vignette, and scanlines. Configurable via config file or CLI flags.

### Architecture

**New struct in `src/render/canvas.rs`:**

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct PostProcessConfig {
    pub bloom: f64,      // 0.0 = off, range 0.0-1.0
    pub vignette: f64,   // 0.0 = off, range 0.0-1.0
    pub scanlines: bool,
}
```

**New method on `Canvas`:**

```rust
pub fn post_process(&mut self, config: &PostProcessConfig)
```

Called after `apply_effects()`, before `canvas.render()`. Runs three passes in order: bloom, then scanlines, then vignette.

### Pass Details

**Bloom:** For each pixel with brightness > 0.6, brighten its 8 neighbors proportionally to bloom intensity. Uses a 3x3 kernel with a weight of `bloom * 0.15` per neighbor. Operates on a temporary copy to avoid cascading within a single pass.

**Vignette:** For each pixel, compute normalized distance from canvas center (0.0 at center, 1.0 at corners). Multiply brightness by `1.0 - (distance^2 * vignette_strength)`. Strength of 0.5 produces a natural falloff.

**Scanlines:** For every other row of sub-cell pixels (even-indexed rows), multiply brightness by 0.7. Creates a CRT-like horizontal line pattern.

### Configuration

**Config.toml** — new `[postproc]` section:

```toml
[postproc]
# bloom = 0.3
# vignette = 0.4
# scanlines = false
```

**CLI flags:** `--bloom <0.0-1.0>`, `--vignette <0.0-1.0>`, `--scanlines`

Merge order: CLI flags override config, config overrides default (all off).

### Files Changed

- `src/render/canvas.rs` — `PostProcessConfig` struct, `post_process()` method
- `src/config.rs` — `PostProcessConfig` field in Config, TOML parsing, config template
- `src/main.rs` — CLI flags, merge logic, call `post_process()` in render loop

---

## Feature 2: GIF Export from Recordings

### Goal

Convert existing `.asciianim` recordings to animated GIF. No external dependencies — hand-written GIF89a encoder with LZW compression.

### Architecture

**New file:** `src/gif.rs` — minimal GIF89a encoder.

**CLI usage:** `termflix --play recording.asciianim --export-gif output.gif`

The `--export-gif` flag is only valid with `--play`. When both are present, instead of playing to the terminal, decode each frame and write GIF.

### Pipeline

1. Load `.asciianim` via existing `Player::load()`
2. For each frame, strip ANSI escape sequences and extract the visible character grid with colors
3. Build a 256-color global palette from all frames using uniform quantization (6x7x6 RGB cube = 252 colors + 4 reserved)
4. Map each frame's pixels to palette indices
5. LZW-compress each frame's index stream
6. Write GIF89a with NETSCAPE2.0 looping extension, graphic control extensions (delay from frame timestamps), and image descriptors

### GIF89a Encoder Details

- **Color quantization:** Uniform 6x7x6 cube mapping. Each RGB channel divided into levels: R=6, G=7, B=6 (prioritizing green for perceived brightness). 252 colors + 4 system colors (black, white, and 2 grays).
- **LZW compression:** Standard variable-width LZW with minimum code size of 8. Clear code at start of each frame.
- **Frame timing:** Convert millisecond timestamps to centiseconds (GIF delay is in 1/100s). Minimum 2cs per frame. Skip frames that would be less than 1cs apart.
- **Frame deduplication:** Compare consecutive frames' index arrays; skip writing identical frames but extend the previous frame's delay.
- **Output size:** Terminal width x terminal height pixels (1:1 character-to-pixel mapping). No scaling.

### ANSI Decoding

Each recorded frame is a string of ANSI escape sequences. The decoder:

1. Tracks cursor position and current foreground color
2. Parses `\x1b[H` (cursor move), `\x1b[38;2;r;g;bm` (truecolor), `\x1b[38;5;Nm` (256-color), `\x1b[m` (reset)
3. Builds a 2D grid of (character, color) per cell
4. Maps each cell's color to the GIF palette

### Error Handling

- Missing `--play` with `--export-gif`: print error suggesting correct usage
- Invalid recording file: propagate existing load errors
- Empty recording: print error "No frames to export"

### Files Changed

- `src/gif.rs` — new file, GIF89a encoder with LZW, color quantization, ANSI decoder
- `src/main.rs` — `--export-gif` CLI flag, export logic when combined with `--play`
- `src/record.rs` — expose frame data (timestamps + content) for GIF conversion

---

## Feature 3: Macro-Based Animation Registration

### Goal

Replace the four-place manual registration pattern with a single `declare_animations!` macro invocation.

### Current Pattern (4 places to touch when adding an animation)

1. `pub mod fire;` — module declaration (~55 lines)
2. `("fire", "Doom-style fire effect...")` — ANIMATIONS list (~80 lines)
3. `"fire",` — ANIMATION_NAMES list (~55 lines)
4. `"fire" => Box::new(fire::Fire::new(width, height)),` — create() match (~60 lines)

### New Pattern

Module declarations remain manual (Rust macro hygiene requires them at module scope). The macro handles ANIMATIONS, ANIMATION_NAMES, and create():

```rust
declare_animations! {
    (fire, fire::Fire, "Doom-style fire effect with heat propagation"),
    (matrix, matrix::Matrix, "Matrix digital rain with trailing drops"),
    // ... all 44 animations
}
```

The macro generates:
- `ANIMATIONS` const slice with `(name, description)` tuples
- `ANIMATION_NAMES` const slice with string names
- `create()` function with match arms calling `$module::$Struct::new(width, height, scale)`

### Constructor Signature

Most animations take `(width, height, scale)`. Some take `(width, height)` and a few take `()`. The macro calls `$module::new(width, height, scale)` for all — animations that don't use `scale` already accept it as a parameter.

### Backward Compatibility

- `Animation` trait, `ANIMATIONS`, `ANIMATION_NAMES`, `create()` all keep the same public API
- Existing tests pass unchanged
- Adding a new animation becomes: add `pub mod` line + one macro entry

### Files Changed

- `src/animations/mod.rs` — replace manual lists with macro invocation

---

## Feature 4: Unified Particle System

### Goal

Extend the shared `ParticleSystem` in `generators/mod.rs` to support per-particle RGB color, then migrate the standalone `particles.rs` animation to use it.

### Current State

- `generators/mod.rs`: Shared `Particle` (x, y, vx, vy, life, max_life) + `ParticleSystem` with gradient-based coloring
- `particles.rs`: Standalone `Particle` struct with per-particle `r, g, b` fields — the only animation not using the shared system

### Changes to `generators/mod.rs`

**Extended `Particle`:**

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

Default color (255, 255, 255) for backward compatibility. Gradient-based `draw()` ignores these fields.

**New methods on `ParticleSystem`:**

- `emit_colored(count, r_range, g_range, b_range)` — emit particles with per-particle random color in given ranges
- `draw_colored(canvas)` — draw using per-particle color with life-based fade

### Migration of `particles.rs`

- Remove standalone `Particle` struct
- Replace `Vec<Particle>` field with `generators::ParticleSystem`
- Move firework spawn logic into `update()` using `emit_colored()`
- Preserve all existing behavior: gravity, drag, fade, timer-based spawning

### Backward Compatibility

- Existing `ParticleSystem` users (rain, smoke, fountain, waterfall, campfire) are unaffected — they use `emit()` + `draw()` which still uses gradient coloring
- `Particle` struct gains `r, g, b` fields initialized to white — no behavior change

### Files Changed

- `src/generators/mod.rs` — extend `Particle`, add `emit_colored()`, `draw_colored()`
- `src/animations/particles.rs` — migrate to shared system

---

## Implementation Order

No dependencies between features. Recommended order by risk and complexity:

1. **Macro-Based Registration** — refactor-only, no behavior change, verified by existing tests
2. **Unified Particle System** — small scope (2 files), easy to verify
3. **Post-Processing Effects** — additive feature, canvas extension
4. **GIF Export** — largest scope, new file + encoder, most testing needed

## Testing Strategy

- **Macro:** Existing tests (`test_create_returns_some_for_all_known_names`, `test_animation_names_match_animations_list`) validate correctness
- **Particle:** Existing `particles` animation must produce identical visual output
- **Post-processing:** Unit tests for each pass with known input/output pixel values
- **GIF:** Round-trip test — record a short animation, export to GIF, verify file header and frame count
