# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
make checkall          # Full gate: fmt-check + lint + typecheck + test + build (same as CI)
make test              # cargo test
make lint              # cargo clippy --all-targets --all-features -- -D warnings
make fmt               # cargo fmt
make build             # cargo build
make run ARGS="fire"   # cargo run --release -- fire
make hooks             # install the pre-commit hook (sets core.hooksPath=.githooks)
```

Pre-commit hook (`.githooks/pre-commit`) runs `make checkall`. Run `make hooks` once after
cloning — without it `core.hooksPath` points at `.git/hooks` and the hook never fires.

Run a single test: `cargo test test_name` (e.g., `cargo test test_create_returns_some`)

Golden-frame tests hash every animation's deterministic output against per-OS files
(`golden_hashes.txt` is macOS, plus `golden_hashes_linux.txt` and
`golden_hashes_windows.txt`) — `globe`'s land/grid membership tests flip a few pixels
between Apple libm and glibc/ucrt via asin/atan2. The test selects the right file at
compile time; an intentional visual change requires regenerating the goldens on the
platform whose file changed and reviewing their diff like any other code change:
`TERMFLIX_UPDATE_GOLDEN=1 cargo test golden`

## Architecture

**Pure synchronous Rust** (edition 2024, requires Rust 1.88+, per Cargo.toml rust-version). No async runtime. One optional background thread for external control file watching.

### Core Pipeline

```
pipeline::produce_frame(anim, canvas, dt, t, &FrameEffects)   // clear → update → smoothing → effects → color assist → post-process
→ canvas.render() → libc::write()
```

Live loop, gallery, and encoder benchmarks all go through `src/render/pipeline.rs::produce_frame` — one place owns clearing and post-effects; animations receive a cleared canvas. Animations write to a **mode-agnostic pixel buffer** (`Canvas`) using sub-cell coordinates. The render step converts pixels to terminal characters based on the active render mode (Braille 2x4, HalfBlock 1x2, or ASCII density).

### Module Layout

- `src/main.rs` — CLI (clap derive), `run_loop` event loop, keybindings, terminal restore
- `src/animations/mod.rs` — `Animation` trait, `declare_animations!` macro, factory function
- `src/animations/*.rs` — 55 individual animation implementations
- `src/render/canvas.rs` — `Canvas` struct (pixel/color buffers), post-processing (bloom, vignette, scanlines)
- `src/render/braille.rs` / `halfblock.rs` — Render mode implementations
- `src/generators/mod.rs` — Shared `ParticleSystem`, `ColorGradient`, `EmitterConfig`
- `src/color.rs` — Shared `hsv_to_rgb` (hue wraps via `rem_euclid(1.0)`)
- `src/config.rs` — TOML config loading (`dirs::config_dir()/termflix/config.toml`, per-OS)
- `src/external.rs` — ndjson external control (stdin or file watcher)
- `src/record.rs` — Frame recording/playback (`.asciianim` format)
- `src/gif.rs` — Hand-written GIF89a encoder for export

### Adding a New Animation

1. Create `src/animations/your_anim.rs` implementing the `Animation` trait
2. Add `pub mod your_anim;` in `src/animations/mod.rs`
3. Add `("your_anim", your_anim::YourAnim, "Description")` to the `declare_animations!` macro invocation

The macro generates `ANIMATIONS`, `ANIMATION_NAMES`, and the `create()` factory function automatically.

### Key Design Decisions

- **`event::poll()` as frame timer** — yields to OS for signal handling instead of `thread::sleep`
- **Chunked `libc::write()` on Unix** — 16KB chunks with inter-chunk quit checks for responsive exit even when tmux buffer is full
- **Labeled loop `'outer`** in `run_loop` — enables profile summary output on any exit path
- **Pre-commit hook runs `checkall`** — all commits must pass fmt, clippy (warnings as errors), check, test, and build

### Animation Trait

```rust
pub trait Animation {
    fn name(&self) -> &str;
    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64);
    fn preferred_render(&self) -> RenderMode { RenderMode::HalfBlock }
    fn set_params(&mut self, _params: &ExternalParams) {}
    fn on_resize(&mut self, _width: usize, _height: usize) {}
    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] { &[] }
}
```

Constructor signature: `fn new(width: usize, height: usize, scale: f64) -> Self`

## CI

GitHub Actions, manual trigger only. Test matrix: ubuntu/macos/windows. Lint on ubuntu. Release workflow builds 5 cross-platform targets + publishes to crates.io.
