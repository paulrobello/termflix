# Hacker News Release

## Title (80 char limit: 67/80)
Show HN: Termflix – 43 procedural animations in your terminal, written in Rust

## URL
https://github.com/paulrobello/termflix

## Text (2000 char limit: ~1900/2000)

Hey HN! I built Termflix, a terminal animation player with 43 procedurally generated animations — fire, matrix rain, Mandelbrot zoom, boids, Game of Life, black hole, sorting visualizer, and more. Zero dependencies beyond a terminal.

**What it does:**

• 43 animations across physics sims, fractals, cellular automata, and demo-scene classics

• 3 render modes: Braille (2×4 pixels/cell), half-block, and ASCII — each animation picks its best default

• 4 color modes: Mono, ANSI 16, ANSI 256, and 24-bit true color

• TOML config file for persistent defaults (`--init-config`, `--show-config`)

• Color quantization option for slow terminals or tmux (`color_quant` in config)

• Runtime hotkeys: cycle animations, render modes, color modes without restarting

• Recording & playback: record sessions as `.termflix` files and replay them

**Technical highlights:**

The renderer uses a pixel-level canvas mapped to terminal characters. Braille mode packs 2×4 sub-cell pixels into a single character. The output path does color deduplication (only emits escape sequences when color changes from the previous cell) and writes a single buffer per frame via a single `write` + `flush` — dramatically reducing output volume.

In tmux, adaptive frame pacing measures actual throughput and scales FPS to match what tmux can drain, preventing the output backlog that causes input lag and beachball freezes. Frames are written in 16KB chunks with keyboard polling between each so quit is always responsive.

Physics sims include: boids flocking, Conway's Game of Life, diffusion-limited aggregation (crystallize), Langton's ant, and an AI-controlled Snake.

**Installation:**

```bash
curl -sL https://raw.githubusercontent.com/paulrobello/termflix/main/install.sh | bash
# or
cargo install termflix
```

Pre-built binaries for Linux x86_64/ARM64, macOS x86_64/Apple Silicon, and Windows on the releases page. MIT licensed.

Built with: Rust, crossterm
