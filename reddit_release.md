# Termflix – 43 Procedural Animations in Your Terminal, Written in Rust

I'm releasing **Termflix**, a terminal animation player with 43 procedurally generated animations. Zero dependencies beyond a terminal — runs on Linux, macOS, and Windows.

## What Is It?

43 procedurally generated animations rendered with Unicode characters. The renderer uses a pixel-level canvas mapped to terminal characters:

- **Braille mode** (`⠁⠂⠃...⣿`) — 2×4 sub-pixels per cell, highest resolution
- **Half-block mode** (`▀▄█`) — 1×2 pixels per cell, good color+resolution balance
- **ASCII mode** (` .:-=+*#%@`) — 1×1, widest compatibility

Each animation picks its best default render mode automatically.

## Animations (43 total)

**Nature & Weather:**
Fire, matrix rain, plasma, starfield (3D), ocean waves, aurora borealis, lightning (recursive branching), smoke, ripple, snow, fireflies, petals (cherry blossom), campfire, waterfall, sandstorm, eclipse, black hole

**Science & Math:**
Conway's Game of Life, boids flocking, Langton's ant, DNA helix, diffusion-limited aggregation (crystallize), sorting visualizer, atom model, rotating globe, Mandelbrot zoom, dragon curve fractal, Sierpinski triangle

**Games & Demos:**
Snake (AI-controlled), Space Invaders demo, Pong (AI vs AI), hackerman terminal, audio visualizer, lava lamp, radar sweep, pulse rings, spiral arms, particle fountain, rain with splashes, water fountain, fluid flow field, petri dish cells

## Features

- **3 Render Modes** — Braille, half-block, ASCII
- **4 Color Modes** — Mono, ANSI 16, ANSI 256, 24-bit true color
- **Config file** — TOML persistent defaults (`--init-config` / `--show-config`)
- **Runtime Hotkeys** — cycle animations, render modes, color modes without restarting
- **Recording & Playback** — record to `.asciianim` files, replay later
- **Auto-Cycle** — rotate through all animations on a configurable timer
- **Unlimited FPS** — `--unlimited` removes the frame cap, renders as fast as the terminal accepts
- **Low CPU** — color dedup + single buffered write per frame keeps it light

## tmux Support

termflix detects tmux and adapts automatically:
- **Adaptive frame pacing** — measures tmux's actual throughput and adjusts FPS to prevent the output backlog that causes input lag and beachball freezes
- **Instant quit** — no waiting minutes for buffered frames to drain
- **Split-safe** — no lockups when splitting panes; FPS scales with pane size

Typical FPS (200×44, halfblock truecolor): ~10 fps full pane, ~20 fps split, 24 fps outside tmux.

## Config

```bash
termflix --init-config   # generate config file
termflix --show-config   # show path and current settings
```

```toml
animation = "plasma"
render = "half-block"
color = "true-color"
fps = 24
color_quant = 0   # 4/8/16 for slower terminals
unlimited_fps = false  # remove FPS cap
```

## Installation

```bash
# Quick install (Linux / macOS / WSL)
curl -sL https://raw.githubusercontent.com/paulrobello/termflix/main/install.sh | bash

# From crates.io
cargo install termflix

# Pre-built binaries (Linux x86_64/ARM64, macOS x86_64/Apple Silicon, Windows x86_64)
# → https://github.com/paulrobello/termflix/releases/latest
```

## Usage

```bash
termflix                        # Default animation (fire)
termflix mandelbrot          # Specific animation
termflix --list                 # List all 43 animations
termflix plasma -r braille   # Force render mode
termflix --cycle 10             # Auto-cycle every 10 seconds
termflix --clean                # No status bar
termflix matrix --record s.asciianim  # Record session
termflix --play s.asciianim      # Replay recording
```

## Links

- GitHub: [github.com/paulrobello/termflix](https://github.com/paulrobello/termflix) — MIT licensed
- crates.io: [crates.io/crates/termflix](https://crates.io/crates/termflix)

Contributions welcome! Implementing the `Animation` trait + registering in `mod.rs` is all it takes to add a new animation.

---

*Built with: Rust, crossterm*
