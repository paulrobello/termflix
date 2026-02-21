# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

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
