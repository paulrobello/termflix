# Design: Dead Code Cleanup, Exposed Params, Transitions, Keybindings

Date: 2026-05-06

## Overview

Four independent features from ideas.md, implemented in dependency order.

---

## 1. Dead Code Cleanup — Remove `vortex`

`src/animations/vortex.rs` is completely disconnected from the build. No `pub mod vortex;` declaration, no entries in `ANIMATIONS`/`ANIMATION_NAMES`/`create()`. The file is dead code the compiler never sees.

**Action:** Delete `src/animations/vortex.rs`. No other files reference it.

---

## 2. Per-Animation Exposed Parameters

Expose `set_params()` / `supported_params()` on 5 more animations, reusing existing `ExternalParams` fields (`intensity`, `color_shift`, `speed`).

| Animation | `intensity` maps to | `color_shift` maps to | `speed` maps to |
|-----------|---------------------|-----------------------|-----------------|
| `boids` | cohesion strength | separation distance | — |
| `particles` | gravity | trail decay rate | — |
| `wave` | amplitude | frequency multiplier | — |
| `sort` | — | — | step delay |
| `snake` | — | — | tick rate |
| `pong` | — | — | tick rate |

Each animation gets two method overrides:
- `supported_params()` returns `&[(&str, f64, f64)]` with param name, min, max
- `set_params()` reads from `ExternalParams` fields and clamps to internal ranges

No changes to `ExternalParams` struct or external protocol.

---

## 3. Transition Effects Between Animations

Fade-out/fade-in when switching animations (hotkey, auto-cycle, external).

### State Machine

```rust
enum TransitionState {
    None,
    FadingOut { next_anim_index: usize, remaining: u8 },
    FadingIn { remaining: u8 },
}
```

Constants: `TRANSITION_FRAMES = 8`, `TRANSITION_FADE_FRAMES = 8`.

### Behavior

1. On animation switch request: enter `FadingOut` with `remaining = 8`
2. Each frame during fade-out: multiply intensity by `remaining / 8`, decrement
3. When `remaining == 0`: create new animation, enter `FadingIn` with `remaining = 8`
4. Each frame during fade-in: multiply intensity by `(8 - remaining) / 8`, decrement
5. When fade-in `remaining == 0`: return to `None`

### Integration Point

In `run_loop()`, after `canvas.apply_effects(intensity, hue_shift)` — scale intensity by the transition factor. The canvas pipeline stays unchanged; only the intensity multiplier varies.

Switch points that trigger transitions:
- Next/prev hotkey
- Auto-cycle timer
- External animation change

Skip transition for initial animation load and resize rebuilds.

---

## 4. Configurable Keybindings

### Config Schema

New `[keybindings]` section in `config.toml`:

```toml
[keybindings]
next = "Right"     # default: Right or n
prev = "Left"      # default: Left or p
quit = "q"         # default: q or Esc
render = "r"       # default: r
color = "c"        # default: c
status = "h"       # default: h
```

### Key String Format

- Single chars: `"q"`, `"n"`, `"p"`
- Special keys: `"Right"`, `"Left"`, `"Up"`, `"Down"`, `"Esc"`, `"Enter"`, `"Space"`, `"Tab"`
- Modifiers: `"Ctrl+c"`, `"Alt+q"` (only if needed later)

### Implementation

1. Add `keybindings: Option<HashMap<String, String>>` to `Config` struct
2. `parse_key_binding(s: &str) -> Option<(KeyCode, KeyModifiers)>` — maps string to crossterm types
3. `KeyBindings` struct with resolved `KeyEvent` fields, built from config + defaults
4. In hotkey match: check `KeyBindings` first, fall back to hardcoded defaults
5. All existing hotkeys remain defaults — no breaking change

### Affected Files

- `src/config.rs` — add `keybindings` field, update template
- `src/main.rs` — parse keybindings at startup, use in event match
