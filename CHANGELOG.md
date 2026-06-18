# Changelog

All notable changes to this project will be documented in this file.

## [0.7.2] - 2026-06-18

### Fixed
- **Screensaver dismisses on unbound keys with `--screensaver-keys`** — Previously, with `--screensaver-keys` active, only the `quit` keybind (or `Ctrl+C`) dismissed the screensaver; every other unbound keypress was silently ignored. Now any key that isn't a bound keybinding (or a live hotkey like `b`/`s`/`d`) dismisses it, matching the expectation that a screen saver exits on arbitrary input. Bound keybindings (`next`/`prev`/`render`/`color`/`status`) still work without dismissing. Plain `--screensaver` (without `--screensaver-keys`) is unchanged — any keypress still dismisses.

## [0.7.1] - 2026-06-17

### Changed
- **Documentation refresh** — Synced `ARCHITECTURE.md`, `EXTERNAL_ANIMATION.md`, and `README.md` to the v0.7.0 implementation: corrected the animation count (54 → 60), fixed the render pipeline and `map_color` signature, documented the new `color_assist` module / dirty-cell rendering / threaded writer, and fixed the config example key `unlimited` → `unlimited_fps` (the old name was silently ignored by serde).
- **README** — Documented additional stable options: post-processing flags (`--bloom-intensity` / `--bloom-threshold` / `--vignette` / `--scanlines`), `--profile`, `--full-frames`, `--single-threaded`, `--data-file`, the `n` / `p` hotkey aliases, and the `[postproc]` / `[keybindings]` / `smoothing` / `color_quant` config options.

### Dependencies
- `cargo update` — transitive refresh: `getrandom` 0.4.2 → 0.4.3, `syn` 2.0.117 → 2.0.118; pruned ~18 orphaned transitive crates. No direct-dependency changes.

### Fixed
- **Windows build** — The writer-thread module (`render_sink`) used unix-only APIs (`RawFd`, `libc::write`) without a `cfg(unix)` gate, breaking the Windows release build (a regression introduced when threaded rendering landed after 0.6.0 — it had never been built on Windows since). It is now gated to unix; on Windows, frames are written inline via `stdout.write_all()` (the existing non-threaded path). Verified with `cargo check --target x86_64-pc-windows-msvc`.

## [0.7.0] - 2026-06-17

### Added
- **Ordered (Bayer) dithering for ANSI-256** — In `ansi256` color mode, `--dither` (or `dither = true` in config, or the live `d` hotkey) applies 4×4 Bayer-matrix ordered dithering so gradients and glows look near-true-color on 256-color terminals (SSH sessions, older terminals). Off by default; non-dithered output is byte-identical to before.
- **Colorblind-safe color assist** — Two opt-in, mutually exclusive modes selectable via flag or config (the status bar reports the active mode):
  - **`--palette <name>`** — Remaps every animation onto a perceptually-uniform, colorblind-safe gradient (`viridis`, `magma`, `inferno`, `plasma`, `okabe-ito`) keyed by perceived luminance, keeping `plasma`/`aurora`/`boids`/etc. legible for deuteranopia/protanopia users.
  - **`--colorblind <type>`** — Daltonization (Viénot/Brettel LMS-space correction) that nudges each animation's own colors to stay distinguishable for `protanopia`, `deuteranopia`, or `tritanopia`.

## [0.6.0] - 2026-06-15

### Added
- **`--screensaver-keys` flag** — When combined with `--screensaver`, keeps all keybindings active instead of exiting on any keypress. The screensaver stays fully interactive: `next`/`prev` cycle animations, and the `render`/`color`/`status`/bloom hotkeys all work; only the `quit` keybind (or `Ctrl+C`) dismisses it. `FocusGained` no longer auto-exits in this mode either. Requires `--screensaver` (enforced by clap).

## [0.5.1] - 2026-05-07

### Added
- **`--gallery` flag** — Captures every animation (or a comma-separated subset) to a directory as a 640×400 PNG still and a 640×400 animated GIF, plus an `index.html` lightbox gallery. Configurable via `--gallery-dir`, `--gallery-cols`, `--gallery-rows`, `--gallery-wait` (PNG capture frame), and `--gallery-duration` (GIF length).
- **`make gallery` target** — Convenience wrapper around `cargo run --release -- --gallery`. Forwards `ARGS`, e.g. `make gallery ARGS="fire,plasma"`.
- **GitHub Pages gallery deploy** — New `Gallery` workflow (`.github/workflows/gallery.yml`) runs `make gallery` on push to `main` (when `src/`, `Cargo.{toml,lock}`, or `Makefile` change) and on manual dispatch, then publishes the output to GitHub Pages. Live site: <https://paulrobello.github.io/termflix/>.

### Fixed
- **PNG chunk CRC32 was wrong** for chunks with data (IHDR / IDAT). The previous code finalized the CRC over the chunk type and then continued accumulating data without un-/re-finalizing, producing a checksum that strict PNG decoders reject as corrupted. `write_chunk` now computes the CRC over `chunk_type ++ data` in a single pass with one finalize. Added a roundtrip test that walks every chunk in a generated PNG and verifies its CRC.
- **GIF LZW width-bump off-by-one** — The encoder bumped code width one step before the decoder did (`next_code > max_code` instead of `next_code > 1<<width`), which corrupted every frame past the first width transition: the top ~25 rows decoded correctly and the rest went black. Now uses the standard giflib/Pillow rule, with matching roundtrip tests covering pseudo-random data, long compressible runs, and the dictionary-fill / reset path.
- **Gallery GIF colors / resolution** — The gallery's GIF path went through `VirtualTerminal`, which dropped half-block background colors (`48;…`) and could clobber the foreground to black when SGR `0` digits appeared inside combined fg+bg sequences. The path now bypasses `VirtualTerminal` entirely via a new `gif::export_gif_pixels` that takes per-frame RGB pixel data, computes palette indices at native resolution for cheap dedup, and upscales by nearest-neighbor when emitting LZW. Output is now 640×400 with full color fidelity (was 80×25 grey).

## [0.5.0] - 2026-05-07

### Added
- **10 new animations** (54 total, up from 44):
  - **Maze** (`maze`) — Animated maze generation using recursive backtracking with step-by-step wall carving, BFS flood-fill solving showing explored cells, and highlighted solution path. Resets and generates a new maze after display. Ascii render mode.
  - **Tetris** (`tetris`) — Self-playing Tetris with all 7 tetrominoes, AI piece placement using weighted heuristic scoring (line clears, height, holes, bumpiness), ghost piece preview, line-clear flash animation, next-piece preview HUD, and speed progression. HalfBlock render mode.
  - **Metaballs** (`metaballs`) — Organic blobs merging and splitting using thresholded distance fields. 4–6 bouncing balls with HSV-blended colors, smooth edge fade near threshold, and bright core glow. HalfBlock render mode.
  - **Flappy Bird** (`flappy_bird`) — Self-playing Flappy Bird with AI controller, gravity physics, scrolling pipe obstacles with adaptive gap sizing, collision detection, score display, and game-over reset. HalfBlock render mode.
  - **Automata** (`automata`) — Cellular automata cycling through 6 rulesets (Conway's Life, Highlife, Day & Night, Seeds, Diamoeba, Replicator). Cell age tracking with warm color gradient, double-buffered grid, auto-cycle every 17 seconds with dead-grid early reset. HalfBlock render mode.
  - **Pendulum** (`pendulum`) — Pendulum wave with 20 pendulums at slightly different periods creating mesmerizing phase patterns. Rainbow-colored bobs with DDA-rendered rods, ghost trails, and pivot dots. Pure math, no stored state. HalfBlock render mode.
  - **Voronoi** (`voronoi`) — Animated Voronoi diagram with 8–15 drifting seed points, distance-based brightness dimming, white edge detection at cell boundaries, and periodic Lloyd relaxation toward uniform cell distribution. HalfBlock render mode.
  - **N-Body** (`nbody`) — N-body gravitational simulation with 5–8 orbiting masses, Euler integration with 2 sub-steps per frame, fading color trails, collision/merging with mass-weighted averaging, and respawn when bodies deplete. HalfBlock render mode.
  - **Rainforest** (`rainforest`) — Layered rainforest scene with parallax scrolling across 3 depth layers (background mountains, mid-ground trees, foreground canopy), falling leaves with horizontal sway, periodic rain bursts, and tropical birds. Ascii render mode.
  - **Reaction Diffusion** (`reaction_diffusion`) — Gray-Scott reaction-diffusion system producing organic coral/brain patterns. Downsampled simulation grid (1/4 canvas) for performance, 9-point Laplacian stencil, auto-reseed every 30 seconds. HalfBlock render mode.
- **Matrix rain depth enhancement** — `matrix` animation now renders 3 depth layers (far/dim/slow, mid/medium, near/bright/fast) with different drop counts, speeds, trail lengths, and head colors, creating parallax depth.
- **Post-processing effects** — Bloom/glow, vignette (edge darkening), and CRT scanline effects configurable via `--bloom-intensity`, `--bloom-threshold`, `--vignette`, `--scanlines` CLI flags or `[postproc]` section in config. Press `b` to toggle bloom at runtime.
- **GIF export** — `--export-gif output.gif` (requires `--play`) converts `.asciianim` recordings to animated GIF with hand-written GIF89a encoder (no new dependencies).
- **Per-animation exposed parameters** — 6 more animations now respond to external control (`set_params`/`supported_params`): `boids` (intensity→cohesion, color_shift→separation), `particles` (intensity→gravity, color_shift→drag), `wave` (intensity→amplitude, color_shift→frequency), `sort` (speed→ops/frame), `snake` (speed→tick rate), `pong` (speed→ball speed). Total: 8 animations with external params (up from 2).
- **Transition effects** — 8-frame fade-out/fade-in when switching animations via hotkey, auto-cycle, or external control, replacing the previous instant cut.
- **Configurable keybindings** — `[keybindings]` section in `config.toml` to remap hotkeys (`next`, `prev`, `quit`, `render`, `color`, `status`). Supports single characters, special keys (`Right`, `Left`, `Esc`, `Space`, `Tab`), and modifier combos (`Ctrl+c`, `Alt+q`).
- **`--list` filtering** — `termflix --list fire` shows only animations matching the substring in name or description (case-insensitive). Bare `--list` still shows all.
- **`--profile` mode** — `termflix --profile <anim>` runs the animation and prints a timing summary on exit: avg/min/max/p95 for update time, render time, and total frame time, plus avg FPS.

### Changed
- **Unified particle system** — Extended shared `Particle` struct with per-particle RGB color. Added `emit_colored()` and `draw_colored()` methods. Migrated `particles` animation from standalone to shared system.
- **Animation constructor standardization** — All animation `new()` methods now accept `(width, height, scale)` for consistency with the macro registration system.
- **Macro-based animation registration** — `declare_animations!` macro replaces manual 3-list registration pattern. Adding a new animation now requires one macro entry instead of editing 4 places.

### Removed
- **Dead `vortex` animation** — Removed `vortex.rs` which was never compiled or registered (no `pub mod vortex;` declaration).

### Dependencies
- Updated `clap` 4.5 → 4.6
- Updated `notify` 7 → 8 (no API changes for our usage)
- Updated `toml` 1.0 → 1.1
- Loosened pin on `serde`, `serde_json`, `libc`, `rand`, `dirs` to semver-compatible ranges

### Fixed
- **Maze black gaps** — Walls and passages now render as solid colored pixels using cell pitch of 2, eliminating visible gaps in half-block mode.
- **Rainforest density** — Default render mode changed to half-block. Vegetation density increased (~2x foreground trees, ~1.7x mid-ground trees, ~2x ferns).
- **Terminal restore on exit** — Properly restores terminal state after animation exit, preventing leftover raw mode or alt screen issues.
- Collapsible `match` arm for `Event::FocusGained` in screensaver mode (clippy `collapsible_match`)

### CI
- Updated GitHub Actions to latest major versions

## [0.4.2] - 2026-02-26

### Added
- **Garden animation** (`garden`) — 44th animation. ASCII garden scene with six plant varieties (rose, daisy, tulip, tree, sunflower, fantasy) that only grow when raindrops hit them. Features a stationary starburst sun in the top-right corner, drifting clouds that randomly trigger rain bursts, raindrop splash effects, and per-plant randomised height. Rose stems interleave plain `|` and leafed `|~` rows for natural variety. Garden resets 60 seconds after all plants reach full bloom; ~25% of spots are randomly left bare each cycle for a natural, uneven look.
- **External visual control** — Control the running animation over stdin (when stdin is not a TTY) or a watched file (`--data-file PATH`) using newline-delimited JSON. Supported fields: `animation`, `speed`, `intensity`, `color_shift`. `fire` and `plasma` respond to `intensity` and `color_shift` respectively.
- **`supported_params()` on Animation trait** — Each animation can advertise which external parameters it handles, enabling future `--list-params <animation>` introspection.
- **`on_resize` trait hook** — Animations implement `on_resize(width, height)` instead of detecting dimension changes inside `update()`. Eliminates a per-frame check from all 43 prior animations.
- **Inline test suites** — Unit tests for config round-trip, `ExternalParams` JSON parsing, base64 encode/decode, animation factory, and canvas bounds/color operations.

### Performance
- **Boids O(N) neighbour search** — Replaced the O(N²) all-pairs loop with a spatial hash grid (cell size = perception radius). Neighbour lookup now checks only the 9 adjacent cells, cutting comparisons from ~90 000 to ~300 per frame at default scale.
- **RNG moved to struct fields** — Eliminated per-frame `rand::rng()` calls across all animations that used randomness in `update()`.
- **Raindrop colors precomputed** — `rain`, `particles`, and `fountain` now compute particle RGB at spawn time instead of every frame.

### Fixed
- **File watcher panics** — `--data-file` with a missing path or exhausted inotify limit no longer panics inside raw mode; errors are logged to stderr and the watcher is silently skipped.
- **`animations::create()` panic** — Factory function now returns `Option<Box<dyn Animation>>` instead of `panic!`-ing on unknown names. Callers handle `None` cleanly.
- **Garden right-side gap** — Integer-division accumulation in column placement left the rightmost ~10% of the screen unplanted. Switched to floating-point even distribution.
- **Tree `|` above `Y`** — Separate `Y` and `\|/` rows in the tree shape produced a trunk character above the fork during partial growth. Merged into a single `\Y/` row; canopy widened to 5 / 7 `W` characters.
- **Redundant bounds checks** removed from animations that double-guarded coordinates already bounds-checked by `Canvas::set_*`.
- **Render threshold magic numbers** documented as named constants; Braille `unwrap_or` replaced with `unwrap()` plus invariant comment.
- **`serde_json`** pinned to `"1.0"` (was loose `"1"`).
- **`notify` crate** gains `mio` feature for correct Linux inotify support (was kqueue-only, fell back to polling on Linux).

### Docs
- `ARCHITECTURE.md` — full system design reference covering the animation trait, canvas/render pipeline, external control protocol, and frame pacing.
- `EXTERNAL_ANIMATION.md` — user guide for the stdin/file JSON control protocol.

## [0.4.1] - 2026-02-25

### Fixed
- **Unknown animation name causes panic** — passing an unrecognised animation name (e.g. `termflix bogus`) previously triggered a Rust panic and left the terminal in raw mode. The name is now validated before raw mode is enabled; an invalid name prints a clean error message with the full list of available animations and exits with code 1.

## [0.4.0] - 2026-02-24

### Added
- **Screensaver mode** — `--screensaver` flag exits immediately on any keypress or when the terminal window gains focus. Useful for desktop screensaver integrations (e.g. `termflix matrix --clean --screensaver`).

## [0.3.0] - 2026-02-23

### Added
- **Unlimited FPS mode** — `--unlimited` flag (and `unlimited_fps` config option) removes the FPS cap. Renders as fast as the terminal can actually accept data (adaptive pacing still active). Status bar shows `∞ fps`.

### Fixed
- **Unlimited mode quit hang** — without adaptive pacing, unlimited mode flooded the terminal faster than it could drain, causing `libc::write()` to block for seconds and making `q` unresponsive. Adaptive pacing is now enabled in unlimited mode for all terminals. Added a post-write quit check as defense-in-depth for the EMA warmup period.
- **Terminal stuck in alt screen after exit** — every frame starts with `\x1b[?2026h` (BSU begin synchronized output). If quit fires mid-write-loop, the terminal receives the begin marker but never the end marker (`\x1b[?2026l`), entering sync mode and buffering all subsequent data including the restore sequences. Prepending `\x1b[?2026l` to the restore sequence closes any pending sync block before restoring cursor and leaving alt screen. Also removed `O_NONBLOCK` from the restore write (was silently dropping sequences via `EAGAIN` if buffer was full).

## [0.2.0] - 2026-02-21

### Added
- **Config file support** — TOML config file for persistent defaults (`--init-config`, `--show-config`)
- **Color quantization option** — `color_quant` setting for slower terminals (reduces output at cost of color precision)

### Performance
- **Adaptive frame pacing in tmux** — Automatically adjusts frame rate to match tmux's actual throughput, preventing output backlog that caused choppiness, input lag, and beachball freezes
- **Color deduplication** — Only emits color escape sequences when the color actually changes from the previous cell, dramatically reducing output volume for all render modes
- **Buffered output** — Single `write` + `flush` per frame via manual buffer instead of per-cell `execute!()` calls
- **Chunked writes with quit detection** — Frames written in 16KB chunks with keyboard polling between each, so quit is always responsive even under heavy output

### Fixed
- **tmux split lockup** — Removed blocking clear screen on resize that deadlocked when tmux's output buffer was full
- **tmux quit delay** — Was taking 1+ minute to quit in tmux as buffered frames drained. Now runs `tmux clear-history` + `refresh-client` on exit for instant cleanup
- **iTerm2 lockup** — Large terminals (200×50+) through tmux were generating ~5MB/s of escape sequences, overwhelming iTerm2's renderer
- **Status bar wrapping** — Truncated to terminal width to prevent line wrapping artifacts
- **Resize handling** — Proper cooldown and frame discarding during terminal resize events
- **Double key registration** — Filter to KeyPress events only (was firing on both press and release)
- **Windows build** — cfg-gate Unix-only chunked writes for cross-platform compatibility

### Changed
- Default FPS reduced from 30 to 24 (sufficient for these animations, reduces terminal load)
- All clippy warnings resolved

## [0.1.0] - 2026-02-20

### Added
- Initial release with 43 procedurally generated animations
- 3 render modes: braille (2×4 per cell), half-block (1×2), ASCII density
- 4 color modes: mono, ANSI 16, ANSI 256, 24-bit true color
- Per-animation default render mode selection
- Runtime hotkeys: cycle animations (←/→), render mode (r), color mode (c), status bar (h), quit (q)
- Recording and playback support (`.asciianim` format)
- Auto-cycle mode with configurable interval
- Particle scale factor (--scale)
- Clean mode (--clean) for no status bar
- Reusable `ParticleSystem` generator for particle-based animations
- Canvas `set_char()` for text-based animations (hackerman HUD)
- Cross-platform installer script (`install.sh`)
- CI workflow (manual trigger): test on Linux/macOS/Windows, lint
- Release workflow (manual trigger): 5-platform builds, crates.io publish, GitHub release, Discord notification
- Pre-commit hook: fmt → clippy → check → test → build
- Published to crates.io and GitHub releases (all 5 platforms)
