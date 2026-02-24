# termflix

[![Crates.io](https://img.shields.io/crates/v/termflix)](https://crates.io/crates/termflix)
![Runs on Linux | macOS | Windows](https://img.shields.io/badge/runs%20on-Linux%20%7C%20macOS%20%7C%20Windows-blue)
![Arch x86-64 | ARM | AppleSilicon](https://img.shields.io/badge/arch-x86--64%20%7C%20ARM%20%7C%20AppleSilicon-blue)
![License](https://img.shields.io/badge/license-MIT-green)

A terminal animation player with 43 procedurally generated animations, multiple render modes, and true color support. Low CPU impact, works great in tmux, only needs your terminal.

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://buymeacoffee.com/probello3)

![termflix screenshot](https://raw.githubusercontent.com/paulrobello/termflix/main/screenshot.png)

## Features

- **43 Animations** — Fire, matrix rain, starfields, plasma, ocean waves, aurora, lightning, and much more
- **3 Render Modes** — Braille (highest resolution), half-block, and ASCII density mapping
- **4 Color Modes** — Mono, ANSI 16, ANSI 256, and 24-bit true color
- **Per-Animation Defaults** — Each animation auto-selects its best render mode
- **Runtime Hotkeys** — Cycle animations, render modes, and color modes on the fly
- **Recording & Playback** — Record sessions and replay them
- **Auto-Cycle** — Rotate through animations on a timer
- **Particle System** — Reusable procedural particle engine powering many animations
- **Low CPU** — Efficient rendering with minimal resource usage

## Animations

| Animation | Description | Default Render |
|-----------|-------------|---------------|
| `fire` | Doom-style fire effect with heat propagation | Half-block |
| `matrix` | Matrix digital rain with trailing drops | ASCII |
| `plasma` | Classic plasma with overlapping sine waves | Half-block |
| `starfield` | 3D starfield with depth parallax | Braille |
| `wave` | Sine wave interference from moving sources | Half-block |
| `life` | Conway's Game of Life cellular automaton | Half-block |
| `particles` | Fireworks bursting with physics and fade | Half-block |
| `rain` | Raindrops with splash particles and wind | Half-block |
| `fountain` | Water fountain with jets, splashes, and mist | Half-block |
| `flow` | Perlin noise flow field with particle trails | Half-block |
| `spiral` | Rotating multi-arm spiral pattern | Half-block |
| `ocean` | Ocean waves with foam and depth shading | Half-block |
| `aurora` | Aurora borealis with layered curtains | Half-block |
| `lightning` | Lightning bolts with recursive branching | Half-block |
| `smoke` | Smoke rising with Perlin turbulence | Half-block |
| `ripple` | Ripple interference from random drop points | Half-block |
| `snow` | Snowfall with accumulation on the ground | Half-block |
| `fireflies` | Fireflies blinking with warm glow | Half-block |
| `dna` | Rotating DNA double helix with base pairs | Half-block |
| `pulse` | Expanding pulse rings from center | Half-block |
| `boids` | Boids flocking simulation with trails | Half-block |
| `lava` | Lava lamp blobs rising, merging, and splitting | Half-block |
| `sandstorm` | Blowing sand with dune formation | Half-block |
| `petals` | Cherry blossom petals drifting in wind | Half-block |
| `campfire` | Campfire with rising ember sparks | Half-block |
| `waterfall` | Cascading water with mist spray | Half-block |
| `eclipse` | Moon crossing sun with corona rays | Half-block |
| `blackhole` | Black hole with accretion disk and lensing | Half-block |
| `radar` | Rotating radar sweep with fading blips | Half-block |
| `crystallize` | DLA crystal growth from center seed | Braille |
| `hackerman` | Scrolling hex/binary hacker terminal | ASCII |
| `visualizer` | Audio spectrum analyzer with bouncing bars | Half-block |
| `cells` | Cell division and mitosis animation | Half-block |
| `atom` | Electrons orbiting a nucleus in 3D | Half-block |
| `globe` | Rotating wireframe Earth with continents | Half-block |
| `dragon` | Dragon curve fractal with color cycling | Braille |
| `sierpinski` | Animated Sierpinski triangle with zoom | Braille |
| `mandelbrot` | Mandelbrot set with zoom and color cycling | Braille |
| `langton` | Langton's Ant cellular automaton | Half-block |
| `sort` | Sorting algorithm visualizer | Half-block |
| `snake` | Self-playing Snake game AI | Half-block |
| `invaders` | Space Invaders attract mode demo | Half-block |
| `pong` | Self-playing Pong with AI paddles | Half-block |

## Installation

### Quick Install (Linux / macOS / WSL)

```bash
curl -sL https://raw.githubusercontent.com/paulrobello/termflix/main/install.sh | bash
```

Installs the latest release binary to `/usr/local/bin`. Custom install location:

```bash
INSTALL_DIR=~/.local/bin curl -sL https://raw.githubusercontent.com/paulrobello/termflix/main/install.sh | bash
```

### From crates.io

```bash
cargo install termflix
```

### From Source

Requires Rust 1.85+ (edition 2024):

```bash
git clone https://github.com/paulrobello/termflix
cd termflix
make install
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/paulrobello/termflix/releases/latest):

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `termflix-linux-x86_64` |
| Linux ARM64 | `termflix-linux-aarch64` |
| macOS x86_64 | `termflix-macos-x86_64` |
| macOS ARM64 (Apple Silicon) | `termflix-macos-aarch64` |
| Windows x86_64 | `termflix-windows-x86_64.exe` |

**macOS note:** After downloading manually, remove the quarantine flag:
```bash
xattr -cr termflix-macos-*
chmod +x termflix-macos-*
```

## Usage

```bash
# Run default animation (fire)
termflix

# Run a specific animation
termflix starfield

# List all animations
termflix --list

# Set render mode (braille, half-block, ascii)
termflix plasma -r braille

# Set color mode (mono, ansi16, ansi256, true-color)
termflix fire -c true-color

# Auto-cycle through animations every 10 seconds
termflix --cycle 10

# Scale particle density
termflix rain --scale 1.5

# Remove FPS cap (render as fast as terminal allows)
termflix --unlimited

# Clean mode (no status bar)
termflix --clean

# Screensaver mode (exits on any keypress or focus)
termflix matrix --clean --screensaver

# Record a session
termflix matrix --record session.asciianim

# Play back a recording
termflix --play session.asciianim
```

## Hotkeys

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `←` / `→` | Previous / next animation |
| `r` | Cycle render mode (braille → half-block → ascii) |
| `c` | Cycle color mode |
| `h` | Toggle status bar |

## How It Works

termflix uses a pixel-level canvas that gets rendered to terminal characters:

- **Braille mode** (`⠁⠂⠃...⣿`) — 2×4 pixels per terminal cell = highest resolution
- **Half-block mode** (`▀▄█`) — 1×2 pixels per cell = good balance of resolution and color
- **ASCII mode** (` .:-=+*#%@`) — 1×1 pixel per cell = widest compatibility

Each animation implements the `Animation` trait, writing to the canvas at sub-cell resolution. The renderer converts the canvas to terminal escape sequences with cursor positioning (no newlines — eliminates flickering).

A reusable `ParticleSystem` generator powers many of the particle-based animations with configurable emitters, gradients, gravity, and drag.

## tmux Support

termflix auto-detects tmux and adapts:

- **Adaptive frame pacing** — Automatically adjusts frame rate to match tmux's throughput, preventing output backlog and input lag
- **Responsive quit** — Runs `tmux clear-history` on exit to flush buffered output
- **Split-safe** — No lockups when splitting panes; FPS scales with pane size
- **Background-safe** — No output backlog when switching away from iTerm2

Typical FPS in tmux (200×44, halfblock truecolor):
- Full pane: ~10 fps (smooth)
- Split pane: ~20 fps (less output per frame)
- Outside tmux: 24 fps (full speed)

## Configuration

termflix supports a TOML config file for persistent defaults. CLI flags always override config settings.

```bash
# Generate default config file
termflix --init-config

# Show config file path and current settings
termflix --show-config
```

Config location:
- **macOS**: `~/Library/Application Support/termflix/config.toml`
- **Linux**: `~/.config/termflix/config.toml`
- **Windows**: `%APPDATA%\termflix\config.toml`

Example config:
```toml
# Default animation
animation = "plasma"

# Render mode: braille, half-block, ascii
render = "half-block"

# Color mode: mono, ansi16, ansi256, true-color
color = "true-color"

# Target FPS (1-120)
fps = 24

# Scale factor for particle density (0.5-2.0)
scale = 1.0

# Hide status bar
clean = false

# Auto-cycle interval in seconds (0 = disabled)
cycle = 0

# Remove FPS cap and render as fast as possible (overrides fps)
unlimited = false
```

## Contributing

Contributions are welcome! To add a new animation:

1. Create `src/animations/your_animation.rs` implementing the `Animation` trait
2. Register it in `src/animations/mod.rs`
3. Run `cargo build --release` and test

```bash
cargo fmt       # Format code
cargo clippy    # Lint
cargo build     # Build
```

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## Author

Paul Robello — probello@gmail.com

## Links

- **GitHub**: [https://github.com/paulrobello/termflix](https://github.com/paulrobello/termflix)
- **Crates.io**: [https://crates.io/crates/termflix](https://crates.io/crates/termflix)
