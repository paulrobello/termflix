# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
make checkall          # Full pre-commit suite: fmt + lint + typecheck + test + build
make test              # cargo test
make lint              # cargo clippy -- -D warnings (warnings are errors)
make fmt               # cargo fmt
make build             # cargo build
make run ARGS="fire"   # cargo run --release -- fire
```

Pre-commit hook (`.githooks/pre-commit`) enforces all of `checkall`. Git hooks dir is `.githooks/`.

Run a single test: `cargo test test_name` (e.g., `cargo test test_create_returns_some`)

## Architecture

**Pure synchronous Rust** (edition 2024, requires Rust 1.85+). No async runtime. One optional background thread for external control file watching.

### Core Pipeline

```
Animation::update(canvas, dt, time) → canvas.apply_effects() → canvas.post_process() → canvas.render() → libc::write()
```

Animations write to a **mode-agnostic pixel buffer** (`Canvas`) using sub-cell coordinates. The render step converts pixels to terminal characters based on the active render mode (Braille 2x4, HalfBlock 1x2, or ASCII density).

### Module Layout

- `src/main.rs` — CLI (clap derive), `run_loop` event loop, keybindings, terminal restore
- `src/animations/mod.rs` — `Animation` trait, `declare_animations!` macro, factory function
- `src/animations/*.rs` — 55 individual animation implementations
- `src/render/canvas.rs` — `Canvas` struct (pixel/color buffers), post-processing (bloom, vignette, scanlines)
- `src/render/braille.rs` / `halfblock.rs` — Render mode implementations
- `src/generators/mod.rs` — Shared `ParticleSystem`, `ColorGradient`, `EmitterConfig`
- `src/config.rs` — TOML config loading (`~/.config/termflix/config.toml`)
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

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- Before answering architecture or codebase questions, read graphify-out/GRAPH_REPORT.md for god nodes and community structure
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- For cross-module "how does X relate to Y" questions, prefer `graphify query "<question>"`, `graphify path "<A>" "<B>"`, or `graphify explain "<concept>"` over grep — these traverse the graph's EXTRACTED + INFERRED edges instead of scanning files
- After modifying code files in this session, run `graphify update .` to keep the graph current (AST-only, no API cost)
