# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-02-23

### Added
- **Unlimited FPS mode** — `--unlimited` flag (and `unlimited_fps` config option) removes the FPS cap. Renders as fast as the terminal can actually accept data (adaptive pacing still active). Status bar shows `∞ fps`.

### Fixed
- **Unlimited mode quit hang** — without adaptive pacing, unlimited mode flooded the terminal faster than it could drain, causing `libc::write()` to block for seconds and making `q` unresponsive. Adaptive pacing is now enabled in unlimited mode for all terminals. Added a post-write quit check as defense-in-depth for the EMA warmup period.
- **Terminal stuck in alt screen after exit** — restore sequences (`\x1b[?1049l`) were written with `O_NONBLOCK`, which silently dropped them if the PTY buffer was full (returns `EAGAIN` with no error). The terminal does not auto-restore alt screen when the PTY closes. Fixed by removing `O_NONBLOCK`; after `tcflush` empties the kernel buffer the blocking write returns immediately.

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
- Recording and playback support (`.termflix` format)
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
