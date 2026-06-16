<instructions>
Each idea should have a check box next to it can be marked done.
Commit all work after marking idea done.
</instructions>

# termflix Enhancement Ideas

Ideas for enhancing existing functionality or adding new features, organized by category.

---

## New Animations

---

## External Control & Integration

- [ ] **[control] Bidirectional External Protocol (medium)** — Extend the external control protocol to emit status events (current animation, FPS, canvas size, frame count) as NDJSON on stdout or a second file. Enables external dashboards, music sync, and scripting integrations that need feedback. Currently the protocol is one-way (input only).
- [ ] **[control] MIDI/OSC Input (large)** — Accept MIDI or OSC messages to drive animation parameters in real time. Knobs control intensity/color shift, note-on triggers animation changes. Would make termflix a live visual instrument for VJs and performers.
- [ ] **[control] Audio-Reactive Visualizer (large)** — Replace the fake spectrum in `visualizer` with actual audio input (e.g., via CPAL or PulseAudio). Real FFT analysis driving bar heights, color shifts, and animation intensity. Could also modulate other animations' parameters based on beat detection.
- [ ] **[control] Parameter Presets & Sequencer (medium)** — External control currently sets live params only. Add named preset records and a timestamped event timeline (sequencer) so a single NDJSON stream can script an entire show — preset recall, fades, and animation switches on cue. Useful for music sync and reproducible demos.
- [ ] **[control] MQTT Subscriber Input (medium)** — Subscribe to an MQTT topic as an alternative to stdin/file-watcher control. Lets IoT dashboards, phones, or other machines drive termflix over the network with zero custom code.

---

## Rendering & Visual Quality

- [ ] **[render] Alpha Blending / Additive Compositing (medium)** — Add blend modes to the canvas so overlapping effects combine naturally. `set_colored()` currently overwrites. An additive blend mode would enable glow effects, light trails, and transparency — useful for `fire`, `aurora`, `lightning`, and any particle-heavy animation.
- [ ] **[render] Custom Color Palettes (small)** — Allow users to define named color palettes in config (e.g., `cyberpunk = ["#0d0221", "#0f084b", "#26408b", "#a6f0c6", "#f72585"]`). Animations sample from these instead of hardcoded gradients. Makes each animation more personalizable without code changes.
- [ ] **[render] Sixel / Kitty Image Protocol Support (large)** — Output actual pixel graphics via Sixel or Kitty image protocols on supported terminals. Would enable smooth gradients and true anti-aliasing beyond what Braille/half-block can achieve. Fall back to current modes on unsupported terminals.
- [ ] **[render] Smooth Brightness Transitions (small)** — Track previous-frame pixel values and blend toward target brightness over N frames. Eliminates the "flickering pixel" artifact in animations like `fire` and `plasma` where noise causes rapid on/off toggling at the threshold boundary.
- [ ] **[render] Ordered (Bayer) Dithering for ANSI-256 (medium)** — When running in `ansi256` mode, apply ordered/Bayer-matrix dithering so gradients and glows look near-true-color on terminals that can't do 24-bit. Big quality win for SSH sessions and older terminals.
- [ ] **[render] Colorblind-Safe Palettes (small)** — Ship a curated set of colorblind-safe palettes (Okabe-Ito, viridis) selectable via config/flag. Keeps `plasma`, `aurora`, `boids`, etc. legible for deuteranopia/protanopia users.
- [ ] **[render] Chromatic Aberration Post-Process (small)** — A new post-process pass that offsets RGB channels near the screen edges for a subtle lens/CRT look. Composes with the existing bloom/scanlines/vignette passes in `canvas.post_process()`.

---

## Interaction & Input

- [ ] **[input] Mouse Interaction (medium)** — Handle mouse click/drag events from terminals that support them. Click to spawn particles at cursor position, drag to create force fields, scroll to adjust speed. Would make animations feel interactive rather than passive.
- [ ] **[input] Interactive Mode Flag (small)** — Add `--interactive` flag that enables mouse/click interaction for supported animations. Without the flag, animations remain autonomous as today. Keeps the default experience clean while enabling experimentation.
- [ ] **[input] Web Remote Control (large)** — Serve a simple HTTP/WebSocket interface (e.g., on localhost:8080) that provides a web UI for controlling the running animation — parameter sliders, animation picker, color mode toggle. Uses the existing external control system under the hood.

---

## Recording & Playback

- [ ] **[record] Portable Recording Format (medium)** — Current `.asciianim` stores raw ANSI escape sequences tied to a specific terminal size. A pixel-data format (canvas width/height + per-pixel brightness/RGB arrays) would be portable across render modes and terminal sizes. Could replay the same recording in Braille, half-block, or ASCII.
- [ ] **[record] Recording Timestamp Overlay (small)** — Show a timestamp and frame counter overlay during playback for debugging/demo purposes. Optional via `--play --debug` flag.
- [ ] **[record] Optimized & Looping GIF Export (medium)** — The hand-written GIF89a encoder could gain inter-frame LZW differencing (store only changed pixels) and seamless loop metadata. Smaller files, smoother social-media previews from the gallery workflow.
- [ ] **[record] Video Export via FFmpeg Pipe (medium)** — Add `--export-video out.mp4` that pipes raw frames to an embedded `ffmpeg` encoder. Removes the GIF 256-color ceiling for captured clips and enables soundtracks.

---

## Performance & Architecture

- [ ] **[arch] Adaptive Frame Skip on Slow Terminals (medium)** — If measured FPS drops below target for N consecutive frames, skip animation `update` ticks (not just render) to stay responsive. Complements the existing adaptive frame pacing so heavy animations don't backlog slow terminals.

---

## UX & Polish

- [ ] **[ux] Animation Preview Thumbnails (small)** — Add a `--preview` flag that renders a single representative frame of each animation to stdout as a static snapshot, useful for picking an animation without launching the full player.
- [ ] **[ux] Information Overlay (small)** — Add a hotkey (e.g., `i`) that temporarily shows an overlay with the current animation's name, description, and tunable parameters. Disappears after a few seconds or on next keypress.
- [ ] **[ux] Progress Bar for Auto-Cycle (small)** — When auto-cycle is active, show a thin progress bar at the bottom indicating how long until the next animation switch. Gives visual rhythm to the cycle.
- [ ] **[ux] FPS Graph Overlay (small)** — A minimal real-time FPS sparkline in the corner when the status bar is visible. Helps users understand performance characteristics of different animations and render modes.
- [ ] **[ux] `--random` and `--shuffle` (small)** — `--random` picks one animation at launch; `--shuffle` randomizes the auto-cycle order. Tiny, fun, frequently requested.
- [ ] **[ux] Fuzzy Animation Picker / TUI Menu (medium)** — A `--picker` full-screen menu listing all animations with descriptions and fuzzy search, reusing the existing gallery snapshot code for thumbnails. Replaces memorizing names from `--list`.
- [ ] **[ux] Live Scale & FPS Hotkeys (small)** — Cycle render/color already exist on hotkeys. Add `+`/`-` to nudge `--scale` and `[`/`]` to nudge FPS live, with the new values reflected in the status bar and current run.
- [ ] **[ux] "Now Playing" Terminal Title (small)** — Set the terminal/tab title via the OSC 0/2 escape on each cycle so tmux/terminal tabs show the current animation name. Restore the original title on exit.

---

## Configuration

- [ ] **[config] Per-Animation Overrides (medium)** — Allow `config.toml` sections like `[animation.fire] scale = 1.5` / `[animation.plasma] color = "ansi256"` so each animation remembers its preferred settings without CLI flags.
- [ ] **[config] Named Profiles (small)** — `--profile cinema` loads a named preset group (fps, render, color, post-process) from config, so users can swap between "demo / battery-saver / cinema" instantly.

---

## Documentation & CI

- [ ] **[docs] Auto-Sync Animation Count & Table (small)** — The README currently advertises 44 animations but `src/animations/` has 55 — the count is stale and several animations (`flappy_bird`, `maze`, `metaballs`, `nbody`, `pendulum`, `rainforest`, `reaction_diffusion`, `tetris`, `voronoi`, `automata`) are undocumented. Generate the README animation table from the `declare_animations!` registration in CI so docs can never drift again.
- [ ] **[docs] CI Doc/Example Lint (small)** — A CI job that runs each `--list` / `--show-config` / example snippet shown in the README and fails if the documented output drifts from real behavior.

---

## Distribution & Platform

- [ ] **[dist] Homebrew Tap (small)** — Create a Homebrew tap for macOS users: `brew install paulrobello/tap/termflix`. Automate formula updates in the release workflow.
- [ ] **[dist] Nix Flakes (small)** — Add a `flake.nix` for Nix/NixOS users. The Rust build is straightforward to package.
- [ ] **[dist] Animated Shell Prompt (medium)** — Provide a library/API for embedding a small termflix animation into a shell prompt (PS1). A mini-canvas rendered to a single line would add personality to terminal setups.
- [ ] **[dist] Screensaver Desktop Integration (small)** — Document or provide helper scripts for integrating `--screensaver` mode with common screensaver frameworks: xscreensaver (Linux), shell wrapper for macOS, and Windows screensaver host.
- [ ] **[dist] Arch Linux AUR Package (small)** — Publish `-bin` and `-git` AUR packages for `pacman` users; have the release workflow bump the version automatically.
- [ ] **[dist] WASM / Browser Build (large)** — Compile the engine to `wasm32` with a canvas target so the live gallery runs interactively in the browser, not just as static GIFs. Large discoverability win.
- [ ] **[dist] Embeddable Library API (medium)** — Expose a stable Rust library API (and optional C FFI) so other apps can embed a termflix animation as a widget — status-line animations, game menus, TUI backgrounds.

---

## Priority Quick Picks

**Quick wins (< 1 hour):**
- [ ] Information overlay (`i` key)
- [ ] `--random` / `--shuffle` flags
- [ ] Colorblind-safe palettes
- [ ] "Now Playing" terminal title (OSC 0/2)
- [ ] Auto-sync README animation count & table (fix the 44 vs 55 drift)

**Medium effort (1–4 hours):**
- [ ] Custom color palettes in config
- [ ] Progress bar for auto-cycle
- [ ] Per-animation config overrides
- [ ] Ordered (Bayer) dithering for ANSI-256
- [ ] Fuzzy animation picker / TUI menu

**Large effort (full day+):**
- [ ] Audio-reactive visualizer
- [ ] Sixel/Kitty image protocol
- [ ] Web remote control
- [ ] Mouse interaction
- [ ] 3D Mandelbulb raymarched fractal
- [ ] WASM / browser build
