# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

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
