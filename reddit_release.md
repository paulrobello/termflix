# termflix v0.5.1 — 54 terminal animations, now with post-processing and 10 new visuals

**Repo:** https://github.com/paulrobello/termflix
**Live gallery:** https://paulrobello.github.io/termflix/
**crates.io:** https://crates.io/crates/termflix

termflix is a terminal animation player vibe coded for fun: procedurally generated animations rendered with Unicode sub-cell characters (braille, half-block, ASCII), 24-bit color, low CPU, plays nice in tmux. Pure synchronous Rust, no async, no GPU, no web.

The 0.5 series adds **10 new animations (54 total)**, a CRT-style **post-processing pipeline**, smooth **crossfade transitions**, expanded **scriptable parameter control**, and a handful of CLI quality-of-life flags. Live gallery has a still + animated GIF for every one of them.

## 10 new animations

- **maze** — recursive-backtracking maze generation with step-by-step wall carving, then BFS flood-fill solving with the path highlighted, then resets and does it again
- **tetris** — self-playing Tetris, all seven tetrominoes, AI placement using a weighted heuristic (line clears, height, holes, bumpiness), ghost-piece preview, line-clear flash, next-piece HUD
- **flappy_bird** — self-playing Flappy Bird, gravity physics, scrolling pipes with adaptive gap sizing, collision detection, score, game-over reset
- **automata** — cellular automata cycling through six rulesets (Conway's Life, Highlife, Day & Night, Seeds, Diamoeba, Replicator), with cell-age coloring and dead-grid early reset
- **metaballs** — organic blobs merging and splitting via thresholded distance fields, HSV-blended colors, smooth edge fade, bright core glow
- **voronoi** — animated Voronoi diagram with 8–15 drifting seed points, distance-based brightness dimming, white edge detection at cell boundaries, periodic Lloyd relaxation
- **nbody** — gravitational n-body simulation with 5–8 orbiting masses, sub-stepped Euler integration, fading color trails, mass-weighted merging on collision, respawn when bodies deplete
- **pendulum** — pendulum wave with 20 pendulums at slightly different periods producing mesmerizing phase patterns, rainbow bobs, ghost trails, pivot dots — pure math, no stored state
- **rainforest** — layered scene with parallax across three depth layers (mountains / trees / canopy), falling leaves with horizontal sway, periodic rain bursts, tropical birds
- **reaction_diffusion** — Gray–Scott reaction-diffusion producing organic coral / brain patterns, downsampled grid for performance, 9-point Laplacian, auto-reseed every 30 s

The classic **matrix** rain animation also got a depth-layer rework — three layers (far/dim/slow, mid, near/bright/fast) running with different drop counts, speeds, and trail lengths for parallax.

## Post-processing effects

Configurable via CLI flags or the `[postproc]` section of `~/.config/termflix/config.toml`. Applied to the canvas pixel buffer after `update()` and before `render()`, fully decoupled from animation logic.

- `--bloom-intensity` + `--bloom-threshold` — pixels above the brightness threshold spread a soft glow into their 8 neighbors. On by default at 0.4 / 0.6. Toggle live with the `b` key.
- `--vignette` — quadratic-falloff edge darkening based on distance from canvas center.
- `--scanlines` — CRT-style alternating-row darkening.

Stacks with whatever animation you're running. Looks especially good on `fire`, `aurora`, `plasma`, `nbody`, `lightning`.

## Transitions and external control

- **Crossfade transitions** — switching animations (via key, `--auto-cycle`, or external control) now does an 8-frame fade-out / fade-in instead of an abrupt cut.
- **Scriptable parameter control expanded** — six more animations gained semantic external-param support: `boids` (intensity → cohesion, color_shift → separation), `particles` (gravity, drag), `wave` (amplitude, frequency), `sort` (ops/frame), `snake` (tick rate), `pong` (ball speed). Total: 8 animations responding to live ndjson parameter feeds (the `set_params` mechanism documented in [docs/EXTERNAL_ANIMATION.md](https://github.com/paulrobello/termflix/blob/main/docs/EXTERNAL_ANIMATION.md)).

## CLI niceties

- `[keybindings]` section in `config.toml` — remap `next`, `prev`, `quit`, `render`, `color`, `status` to whatever you like; supports modifier combos (`Ctrl+c`, `Alt+q`) and special keys (`Space`, `Esc`, `Tab`, arrows).
- `termflix --list <substr>` — filter the animation list by name or description (case-insensitive). Bare `--list` still shows all.
- `termflix --profile <anim>` — runs the animation and prints a timing summary on exit: avg/min/max/p95 for update time, render time, and total frame time, plus average FPS.
- `termflix --play recording.asciianim --export-gif out.gif` — converts a recorded session to an animated GIF using a hand-written GIF89a encoder (no external image crates).

## Try it

```
cargo install termflix
termflix                        # default: fire animation
termflix --list                 # all 54
termflix matrix
termflix --auto-cycle 10
termflix --vignette 0.4 --scanlines
termflix nbody --bloom-intensity 0.8
```

Or just look at the live gallery: https://paulrobello.github.io/termflix/

Feedback, bug reports, and animation ideas welcome.
