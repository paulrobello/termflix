# termflix Enhancement Ideas

Ideas for enhancing existing functionality or adding new features, organized by category.

---

## New Animations

---

## External Control & Integration

### [control] Bidirectional External Protocol (medium)
Extend the external control protocol to emit status events (current animation, FPS, canvas size, frame count) as NDJSON on stdout or a second file. Enables external dashboards, music sync, and scripting integrations that need feedback. Currently the protocol is one-way (input only).

### [control] MIDI/OSC Input (large)
Accept MIDI or OSC messages to drive animation parameters in real time. Knobs control intensity/color shift, note-on triggers animation changes. Would make termflix a live visual instrument for VJs and performers.

### [control] Audio-Reactive Visualizer (large)
Replace the fake spectrum in `visualizer` with actual audio input (e.g., via CPAL or PulseAudio). Real FFT analysis driving bar heights, color shifts, and animation intensity. Could also modulate other animations' parameters based on beat detection.

### [control] Per-Animation Exposed Parameters (small)
Only `fire` and `plasma` currently implement `set_params()` / `supported_params()`. Expose tunable parameters for more animations:
- `boids`: cohesion, alignment, separation, speed
- `sort`: algorithm selection, array size
- `snake`/`pong`: game speed, AI aggressiveness
- `particles`: burst size, gravity, trail length
- `wave`: frequency, amplitude, source count

---

## Rendering & Visual Quality

### [render] Alpha Blending / Additive Compositing (medium)
Add blend modes to the canvas so overlapping effects combine naturally. `set_colored()` currently overwrites. An additive blend mode would enable glow effects, light trails, and transparency — useful for `fire`, `aurora`, `lightning`, and any particle-heavy animation.

### [render] Custom Color Palettes (small)
Allow users to define named color palettes in config (e.g., `cyberpunk = ["#0d0221", "#0f084b", "#26408b", "#a6f0c6", "#f72585"]`). Animations sample from these instead of hardcoded gradients. Makes each animation more personalizable without code changes.

### [render] Sixel / Kitty Image Protocol Support (large)
Output actual pixel graphics via Sixel or Kitty image protocols on supported terminals. Would enable smooth gradients and true anti-aliasing beyond what Braille/half-block can achieve. Fall back to current modes on unsupported terminals.

### [render] Post-Processing Effects (small)
Add optional post-processing passes to the canvas: bloom/glow (brighten neighbors of bright pixels), vignette (darken edges), scanlines (CRT effect). Configurable via config or CLI flags. The existing `apply_effects()` is a good hook point.

### [render] Smooth Brightness Transitions (small)
Track previous-frame pixel values and blend toward target brightness over N frames. Eliminates the "flickering pixel" artifact in animations like `fire` and `plasma` where noise causes rapid on/off toggling at the threshold boundary.

---

## Interaction & Input

### [input] Mouse Interaction (medium)
Handle mouse click/drag events from terminals that support them. Click to spawn particles at cursor position, drag to create force fields, scroll to adjust speed. Would make animations feel interactive rather than passive.

### [input] Interactive Mode Flag (small)
Add `--interactive` flag that enables mouse/click interaction for supported animations. Without the flag, animations remain autonomous as today. Keeps the default experience clean while enabling experimentation.

### [input] Web Remote Control (large)
Serve a simple HTTP/WebSocket interface (e.g., on localhost:8080) that provides a web UI for controlling the running animation — parameter sliders, animation picker, color mode toggle. Uses the existing external control system under the hood.

---

## Recording & Playback

### [record] Portable Recording Format (medium)
Current `.asciianim` stores raw ANSI escape sequences tied to a specific terminal size. A pixel-data format (canvas width/height + per-pixel brightness/RGB arrays) would be portable across render modes and terminal sizes. Could replay the same recording in Braille, half-block, or ASCII.

### [record] GIF/APNG Export (medium)
Convert recordings to GIF or APNG for sharing. The canvas pixel data is already available per frame — render each frame to an image and compile into an animated image format. Would make it easy to share animations in READMEs, chats, and documentation.

### [record] Recording Timestamp Overlay (small)
Show a timestamp and frame counter overlay during playback for debugging/demo purposes. Optional via `--play --debug` flag.

---

## Performance & Architecture

### [arch] Macro-Based Animation Registration (small)
Replace the manual 3-step registration (module declaration + `ANIMATIONS` list + `create()` match) with a `declare_animation!()` macro that handles all three in one line. Reduces boilerplate and eliminates the risk of adding an animation but forgetting to register it (like the current `vortex` dead code).

### [arch] Threaded Canvas Rendering (large)
Move rendering off the main thread. The main loop currently computes the animation update and renders synchronously. With a double-buffered canvas, update could run on one thread while the previous frame renders on another, potentially doubling throughput.

### [arch] Profile-Guided Animation Tuning (small)
Add a `--profile` mode that measures per-frame update and render times and outputs a summary on exit. Helps identify which animations need optimization and validates that changes improve performance.

### [arch] Unified Particle System (medium)
Merge the standalone particle code in `particles.rs` with the reusable `ParticleSystem` generator. The standalone version has per-particle RGB colors that the shared system lacks. Adding color customization to the shared system would allow all particle-heavy animations to use one engine.

---

## UX & Polish

### [ux] Animation Preview Thumbnails (small)
Add a `--preview` flag that renders a single representative frame of each animation to stdout as a static snapshot, useful for picking an animation without launching the full player.

### [ux] Search / Filter Animation List (small)
Enhance `--list` output with optional filtering: `termflix --list fire` shows only animations matching "fire". Useful when the list grows beyond what fits on screen.

### [ux] Transition Effects Between Animations (small)
When cycling between animations (via hotkey or auto-cycle), add a brief fade/crossfade transition instead of an instant cut. A simple brightness fade-out over 5-10 frames would look much smoother.

### [ux] Information Overlay (small)
Add a hotkey (e.g., `i`) that temporarily shows an overlay with the current animation's name, description, and tunable parameters. Disappears after a few seconds or on next keypress.

### [ux] Configurable Keybindings (medium)
Allow users to remap hotkeys via config file. Some users may want `Space` for next animation instead of `Right`, or `Enter` to quit. Stores keymap in config.toml.

### [ux] Progress Bar for Auto-Cycle (small)
When auto-cycle is active, show a thin progress bar at the bottom indicating how long until the next animation switch. Gives visual rhythm to the cycle.

### [ux] FPS Graph Overlay (small)
A minimal real-time FPS sparkline in the corner when the status bar is visible. Helps users understand performance characteristics of different animations and render modes.

---

## Distribution & Platform

### [dist] Homebrew Tap (small)
Create a Homebrew tap for macOS users: `brew install paulrobello/tap/termflix`. Automate formula updates in the release workflow.

### [dist] Nix Flakes (small)
Add a `flake.nix` for Nix/NixOS users. The Rust build is straightforward to package.

### [dist] Animated Shell Prompt (medium)
Provide a library/API for embedding a small termflix animation into a shell prompt (PS1). A mini-canvas rendered to a single line would add personality to terminal setups.

### [dist] Screensaver Desktop Integration (small)
Document or provide helper scripts for integrating `--screensaver` mode with common screensaver frameworks: xscreensaver (Linux), shell wrapper for macOS, and Windows screensaver host.

---

## Dead Code Cleanup

### [cleanup] Register or Remove `vortex` Animation (small)
`vortex.rs` exists and is declared in `mod.rs` but is not registered in `ANIMATIONS`, `ANIMATION_NAMES`, or `create()`. Either register it as a 56th animation or remove it if incomplete.

---

## Priority Quick Picks

**Quick wins (< 1 hour):**
- Register/remove `vortex` animation
- `--list` filtering (`termflix --list fire`)
- Expose params on 3-5 more animations

**Medium effort (1-4 hours):**
- Macro-based animation registration
- Custom color palettes in config
- Transition effects between animations

**Large effort (full day+):**
- Audio-reactive visualizer
- Sixel/Kitty image protocol
- Web remote control
- GIF/APNG export
- Mouse interaction
