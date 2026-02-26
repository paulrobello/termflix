# termflix — Full Code Audit

**Date:** 2026-02-26
**Version:** 0.4.1
**Auditor:** Claude Code
**Scope:** Architecture, Design Patterns, Security, Code Quality, Documentation

---

## Executive Summary

termflix is a well-engineered terminal animation player with a clean modular architecture, good Rust practices, and thoughtful handling of terminal edge cases. The codebase demonstrates strong separation of concerns and production-quality terminal I/O management. However, several issues require attention before the next release: two panicking unwraps in the file watcher path, a formatting regression in `garden.rs`, an absence of any test suite, and a widespread pattern of code duplication in animation resize logic.

**Overall Rating:** 8 / 10

---

## Build Status

| Check | Status | Notes |
|-------|--------|-------|
| `cargo build --release` | ✅ PASS | 2.0 MB binary |
| `cargo clippy -- -D warnings` | ✅ PASS | Zero warnings |
| `cargo fmt -- --check` | ❌ FAIL | `garden.rs` needs formatting |
| `cargo test` | ⚠️ N/A | No test suite present |

---

## Findings — Prioritized

### 🔴 CRITICAL

---

#### C-1 — Panicking unwraps in file watcher setup

**File:** `src/external.rs`, lines 149, 151
**Category:** Error Handling / Reliability

```rust
let mut watcher = notify::recommended_watcher(move |res| {
    let _ = file_tx.send(res);
}).unwrap();  // ❌ panics on watcher creation failure

notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
    .unwrap();  // ❌ panics if file does not exist or permission denied
```

If the user passes `--data-file` pointing to a path with restricted permissions, or if the `notify` backend fails to initialize (e.g., inotify limit exhausted on Linux), the program panics immediately after entering raw mode. This leaves the terminal in a broken state with no cleanup.

**Fix:** Replace both unwraps with graceful degradation. Log the error to stderr before entering raw mode, and either skip file watching or exit cleanly.

```rust
let mut watcher = match notify::recommended_watcher(move |res| {
    let _ = file_tx.send(res);
}) {
    Ok(w) => w,
    Err(e) => {
        eprintln!("Warning: could not create file watcher: {e}");
        return;
    }
};
if let Err(e) = notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive) {
    eprintln!("Warning: could not watch {}: {e}", path.display());
    return;
}
```

---

#### C-2 — `animations::create` panics on unknown animation name

**File:** `src/animations/mod.rs`, line 211
**Category:** Error Handling

```rust
_ => panic!("Unknown animation: {}", name),
```

The caller in `main.rs` validates the animation name against `ANIMATION_NAMES` before calling `create()`, so this panic is unreachable in normal use. However:

1. Any future caller that bypasses the validation will crash.
2. The API contract is invisible; callers must read the source to learn validation is required.

**Fix:** Return `Result<Box<dyn Animation>, String>` or `Option<Box<dyn Animation>>` from `create()`. Remove the pre-check in `main.rs` and handle the error there.

```rust
pub fn create(name: &str, width: usize, height: usize, scale: f64) -> Option<Box<dyn Animation>> {
    match name {
        "fire" => Some(Box::new(fire::Fire::new(width, height))),
        // ...
        _ => None,
    }
}
```

---

### 🟡 HIGH

---

#### H-1 — Format check failure in `garden.rs`

**File:** `src/animations/garden.rs`
**Category:** Code Quality / CI

`cargo fmt -- --check` fails, meaning `garden.rs` deviates from the project's rustfmt configuration. This blocks `make checkall` and signals the pre-commit workflow is not being enforced consistently.

**Fix:** Run `cargo fmt` and commit the result before any further commits.

---

#### H-2 — No test suite

**Category:** Testing / Reliability

The project has zero tests. `cargo test` exits immediately with no failures only because there is nothing to test. For a binary TUI, unit-testing the render loop is impractical, but the following are directly testable without terminal interaction:

- `Config::load_config()` — roundtrip TOML parsing
- `ExternalParams` JSON deserialization edge cases
- `animations::create()` — all names resolve, unknown name returns None
- `base64_encode` / `base64_decode` in `record.rs` — pure functions
- `Canvas::set()`, `Canvas::set_colored()` — bounds checking
- `Canvas::rotate_hue()` — color math

**Fix:** Add a `[lib]` section to `Cargo.toml` and move logic worth testing into it. Add `tests/` with at minimum smoke tests for config, JSON parsing, and base64.

---

#### H-3 — Boids O(N²) nearest-neighbor search

**File:** `src/animations/boids.rs`, ~lines 60–62
**Category:** Performance

```rust
let positions: Vec<(f64, f64, f64, f64)> = self.boids.iter().map(...).collect();
for (i, boid) in self.boids.iter_mut().enumerate() {
    for (j, &(ox, oy, ovx, ovy)) in positions.iter().enumerate() {
```

At default scale with 300 boids this is 90,000 comparisons per frame. At 60 fps that is 5.4 million comparisons per second. For terminal frame rates this is acceptable today, but it scales quadratically and becomes a bottleneck at higher boid counts or scale factors.

**Fix (short term):** Reduce the neighbor-search radius to exit early when distance exceeds perception range.
**Fix (long term):** Partition space with a 2D grid (cell size = perception radius) and check only adjacent cells, reducing average complexity to O(N).

---

### 🟠 MEDIUM

---

#### M-1 — Resize pattern duplicated in ~90% of animations

**Category:** Code Duplication / Maintainability

Almost every animation begins `update()` with:

```rust
self.width = canvas.width;
self.height = canvas.height;
```

This pattern is copy-pasted across all 43 animations. There is no hook or default trait method to centralise it. When canvas dimensions change mid-animation, animations that also resize internal buffers (e.g., `fire.rs` lines 31–40) must do so manually.

**Fix:** Add an `on_resize` hook to the `Animation` trait:

```rust
pub trait Animation {
    fn on_resize(&mut self, _width: usize, _height: usize) {}
    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64);
}
```

Call it from the main loop whenever the canvas dimensions change, and remove the per-frame assignment from all animations.

---

#### M-2 — Per-frame RNG creation in hot loops

**File:** `src/animations/fire.rs` line 55, and several others
**Category:** Performance

```rust
// fire.rs - inside update(), called 60x per second
let mut rng = rand::rng();
```

`rand::rng()` is not free; it calls into the OS random source or a thread-local seeded generator depending on the platform. Creating it freshly every frame wastes cycles.

**Files affected:** `fire.rs`, `rain.rs`, `garden.rs`, `particles.rs`, `boids.rs`, and others.

**Fix:** Store `rng` as a field:

```rust
struct Fire {
    rng: rand::rngs::ThreadRng,
    // ...
}

impl Fire {
    pub fn new(...) -> Self {
        Self { rng: rand::rng(), ... }
    }
}
```

---

#### M-3 — Magic numbers without documentation

**Category:** Code Quality / Maintainability

The codebase contains numerous unexplained numeric constants:

| File | Value | Context |
|------|-------|---------|
| `braille.rs:20` | `0.3` | Brightness threshold |
| `halfblock.rs:28` | `0.02` | Dark threshold (inconsistent with braille's `0.3`) |
| `canvas.rs:208` | `64` | ANSI16 brightness cutoff |
| `fire.rs:48` | `0.8` | Default heat rate |
| `fire.rs:80` | `0.85 / 0.6 / 0.3` | Color transition thresholds |
| `rain.rs:36` | `15.0 + depth * 50.0` | Speed formula |

The most concerning is the inconsistency between Braille and HalfBlock render thresholds (0.3 vs 0.02), which means an animation tuned for one renderer looks different in the other.

**Fix:** Replace magic numbers with named constants and doc comments:

```rust
/// Minimum pixel brightness to render a braille dot.
/// Values below this are treated as dark/empty.
const BRIGHTNESS_THRESHOLD: f64 = 0.3;
```

Consider unifying the render thresholds behind a single configurable value, or document why they differ.

---

#### M-4 — Redundant bounds checking in animations

**Category:** Code Quality / Duplication

Several animations manually bounds-check coordinates before calling `canvas.set_colored()` or `canvas.set_char()`:

```rust
// garden.rs ~line 231
if px >= 0 && (px as usize) < canvas.width && py >= 0 && (py as usize) < canvas.height {
    canvas.set_char(px as usize, py as usize, ch, r, g, b);
}
```

`Canvas::set()` and `Canvas::set_char()` already perform this check internally (`canvas.rs` ~lines 82–108). The double-checking is not incorrect, but it clutters code and can mislead readers into thinking the canvas methods are unsafe without the guard.

**Fix:** Remove the manual guards and rely on the canvas's own bounds checking. If clip-on-out-of-bounds is the intended behavior (it is, since canvas methods silently skip OOB writes), this is already correct.

---

#### M-5 — Hardcoded per-frame color arithmetic in particle animations

**File:** `src/animations/rain.rs` lines 129–131, similar in `particles.rs`, `fountain.rs`
**Category:** Performance

```rust
let r = (60.0 + 80.0 * drop.depth) as u8;
let g = (80.0 + 90.0 * drop.depth) as u8;
let b = (120.0 + 135.0 * drop.depth) as u8;
```

This float arithmetic runs for every particle every frame. Since `depth` is fixed at particle creation, these values never change.

**Fix:** Precompute and store RGB as fields on the particle struct at creation time, eliminating per-frame floating-point arithmetic.

---

#### M-6 — Inconsistent use of `dt` and `time` parameters

**Category:** Code Quality

Many animations ignore one or both time parameters, inconsistently prefixing them with `_`:

```rust
fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64)  // fire
fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64)  // plasma
fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64)  // rain
```

There is no documented convention for which animations should use wall-clock `time` vs delta `dt`. This makes it harder to reason about whether an animation would behave correctly under variable frame rates.

**Fix:** Add doc comments to the `Animation` trait explaining when each parameter should be used. Consistently prefix genuinely unused parameters with `_`.

---

#### M-7 — External parameter support limited to 2 of 43 animations

**File:** `src/animations/mod.rs` and individual animation files
**Category:** Feature Completeness

`set_params()` is implemented only in `fire.rs` and `plasma.rs`. All other animations silently ignore `ExternalParams`. There is also no way for a caller to discover which parameters an animation supports.

**Fix (minimal):** Add `fn supported_params() -> &'static [&'static str]` returning `&[]` as a default, overridden by animations that handle params. This enables a `--list-params <animation>` CLI subcommand.

**Fix (ideal):** Adopt `intensity`, `color_shift`, and `speed` as universally honored parameters across all animations.

---

### 🟢 LOW

---

#### L-1 — `serde_json` version constraint is too loose

**File:** `Cargo.toml`
**Category:** Dependency Management

```toml
serde_json = "1"
```

This is equivalent to `>=1.0.0, <2.0.0`, which allows any `1.x` patch to update. All other dependencies are pinned to specific minor versions. While Cargo's lock file prevents unintended updates in practice, a missing `Cargo.lock` (library consumers, fresh CI) could pull surprising versions.

**Fix:**
```toml
serde_json = "1.0"
```

---

#### L-2 — `notify` compiled for macOS kqueue only

**File:** `Cargo.toml`
**Category:** Cross-Platform Compatibility

```toml
notify = { version = "7", default-features = false, features = ["macos_kqueue"] }
```

With `default-features = false` and only `macos_kqueue` enabled, the `notify` crate will compile a fallback polling watcher on Linux. This works but polls at an interval, meaning file change detection may lag by several seconds.

**Fix:** Enable the inotify backend conditionally, or restore default features:

```toml
notify = { version = "7", default-features = false, features = ["macos_kqueue", "inotify"] }
```

Or document the polling fallback in the README.

---

#### L-3 — `Vec` allocation inside hot loops in `garden.rs`

**File:** `src/animations/garden.rs`, ~line 352
**Category:** Performance

```rust
for (i, &ch) in ['.', '\'', '.'].iter().enumerate() {
```

An array literal iterated with `iter()` is fine, but several other locations in `garden.rs` construct temporary `Vec`s inside the per-frame update loop. Given garden's complexity (plants, clouds, drops, splashes), heap allocation churn may be visible as frame-time jitter on slow machines.

**Fix:** Profile with `cargo flamegraph` before optimizing. If confirmed, pre-allocate particle pools or switch to fixed-size arrays.

---

#### L-4 — Recording keeps all frames in memory

**File:** `src/record.rs`
**Category:** Memory Usage

`Recorder` stores the entire recording as a `Vec<Frame>` in memory. A long recording session at high resolution produces large amounts of base64-encoded terminal output. No streaming or chunked write is supported.

**Fix (minimal):** Document the limitation with a note on maximum practical recording length.
**Fix (ideal):** Stream frames to disk incrementally during recording rather than buffering in memory.

---

#### L-5 — `from_u32` unwrap in braille renderer

**File:** `src/render/braille.rs`, line 57
**Category:** Code Quality

```rust
let ch = char::from_u32(BRAILLE_OFFSET + bits).unwrap_or(' ');
```

This is safe because `bits` is always in `0x00..=0xFF` and `BRAILLE_OFFSET` is `0x2800`, placing the result in the known-valid Braille Patterns Unicode block. However, `unwrap_or` implies the failure path is plausible, which could confuse readers.

**Fix:** Use a `debug_assert!` to document the invariant and use `unwrap()` directly, or add a compile-time assertion:

```rust
const _: () = assert!(char::from_u32(0x2800 + 0xFF).is_some());
let ch = char::from_u32(BRAILLE_OFFSET + bits).unwrap();
```

---

#### L-6 — No `on_resize` lifecycle for animations with internal buffers

**File:** `src/animations/fire.rs` lines 31–40, and others
**Category:** Code Quality / Correctness

`Fire` rebuilds its heat buffer whenever `self.width != canvas.width || self.height != canvas.height`. This check runs inside the per-frame `update()` and resets the entire fire state on any resize. While functionally correct, the resize detection is buried inside update logic.

This is a broader symptom of the missing `on_resize` hook described in M-1.

---

#### L-7 — `garden.rs` type alias obscures lifetime semantics

**File:** `src/animations/garden.rs`, lines 5–6
**Category:** Code Clarity

```rust
type PRow = &'static [(i32, char, bool)];
```

This alias hides that `PRow` is a borrowed slice with a `'static` lifetime. Readers unfamiliar with the type must trace it back to understand the borrowing model.

**Fix:** Use an explicit struct with a doc comment, or at minimum add an explanatory inline comment.

---

## Architecture Assessment

### Strengths

- **Trait-based plugin system:** `dyn Animation` allows zero-cost hot-swapping of animations at runtime and clean isolation of simulation state.
- **Layered rendering:** The Canvas abstraction cleanly separates simulation (pixel/color buffers) from presentation (Braille/HalfBlock/ASCII backends). Adding a new renderer requires touching only one file.
- **Terminal I/O:** The frame pacing system (EMA-based write-time estimation, adaptive sleep) is sophisticated and correctly handles tmux buffering. The synchronized output (`\x1b[?2026h/l`) markers and their ordering around terminal cleanup are particularly well thought out.
- **External control:** The JSON-lines protocol with `CurrentState`'s take-method pattern cleanly separates "fire once" changes (animation switching) from continuous parameters (speed, intensity).
- **Config priority chain:** CLI > config file > defaults is correctly implemented with no silent overrides.

### Weaknesses

- **Animation trait API surface is too narrow:** No resize hook, no parameter schema, no introspection. Adding new cross-cutting concerns requires editing all 43 animation files.
- **Factory function API is unsafe:** `animations::create()` panics on invalid input rather than returning a Result.
- **Coupling between animations and canvas dimensions:** Animations store their own `width`/`height` fields and must keep them in sync manually every frame, rather than reading from the canvas.

---

## Security Assessment

The attack surface of termflix is very small: it is a local, single-user binary with no network I/O, no privilege escalation, and no external command execution.

| Vector | Finding |
|--------|---------|
| CLI argument injection | Not applicable; clap parses typed args |
| File path traversal | Recording paths are user-supplied but no restrictions needed for a local tool |
| JSON external control | `serde_json` deserializes into a typed struct; no arbitrary code paths |
| Terminal escape injection | All escape sequences are hardcoded constants; not derived from input |
| Supply chain | All dependencies are mature, widely-audited crates |

**Assessment:** No significant security issues. The only hardening opportunity is input validation for `--data-file` (reject paths that don't exist at startup, rather than panicking when the watcher fails — see C-1).

---

## Documentation Assessment

| Item | Status |
|------|--------|
| README.md | ✅ Excellent — complete feature list, hotkeys, config reference, tmux notes |
| Code comments | ✅ Good — especially in `main.rs` around terminal lifecycle edge cases |
| `Animation` trait | ⚠️ No doc comments on `update()`, `preferred_render()`, or `set_params()` |
| `ExternalParams` fields | ⚠️ No doc comments on JSON field meanings or valid ranges |
| CHANGELOG.md | ✅ Present and current |
| ARCHITECTURE.md | ❌ Referenced in git history but absent from HEAD |

---

## Remediation Checklist

### Immediate (before next commit)

- [ ] Run `cargo fmt` and commit the result (fixes H-1 / `garden.rs`)
- [ ] Fix `.unwrap()` calls in `external.rs:149,151` with graceful error handling (C-1)
- [ ] Change `animations::create()` to return `Option<Box<dyn Animation>>` (C-2)

### Short-term (next minor release)

- [ ] Add minimal test suite: config roundtrip, JSON parsing, base64, animation factory (H-2)
- [ ] Store `rng` as a struct field in all animations that use it per-frame (M-2)
- [ ] Precompute particle colors at creation time in `rain.rs`, `particles.rs`, `fountain.rs` (M-5)
- [ ] Replace magic threshold constants with named constants and comments (M-3)
- [ ] Remove redundant bounds checks that duplicate `canvas.set()` behavior (M-4)
- [ ] Pin `serde_json` to `"1.0"` in `Cargo.toml` (L-1)
- [ ] Add `inotify` feature to `notify` dependency or document polling fallback (L-2)

### Medium-term (architecture improvements)

- [ ] Add `on_resize` hook to `Animation` trait; remove duplicate resize pattern from 43 animations (M-1)
- [ ] Add `supported_params()` to `Animation` trait for parameter introspection (M-7)
- [ ] Implement `set_params()` for at least 10 more animations (M-7)
- [ ] Add doc comments to `Animation` trait methods and `ExternalParams` fields (docs)
- [ ] Investigate spatial partitioning for boids neighbor search (H-3)
- [ ] Consider streaming frame writes in `record.rs` for long sessions (L-4)
- [ ] Restore or archive `ARCHITECTURE.md` (docs)

---

## Dependency Versions (at time of audit)

| Crate | Current | Notes |
|-------|---------|-------|
| crossterm | 0.29.0 | ✅ Current stable |
| clap | 4.5.60 | ✅ Current stable |
| rand | 0.10.0 | ✅ Current stable |
| serde | 1.0.228 | ✅ Current stable |
| serde_json | 1 (loose) | ⚠️ Pin to 1.0 |
| notify | 7 | ✅ Current stable; kqueue-only on macOS |
| noise | 0.9.0 | ✅ Current stable |
| toml | 1.0.3 | ✅ Current stable |
| dirs | 6.0.0 | ✅ Current stable |
| libc | 0.2.182 | ✅ Current stable |

---

*End of audit.*
