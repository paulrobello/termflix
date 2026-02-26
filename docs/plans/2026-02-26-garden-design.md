# Garden Animation Design

**Date:** 2026-02-26
**Status:** Approved

## Overview

A garden scene animation where ASCII-art plants grow only when raindrops hit them. A sun drifts across the sky; clouds drift and occasionally rain; rain triggers plant growth one stage at a time.

## Scene Layout

```
[☀ ...]  [(~~~)] <- sun drifts L→R, 1-3 clouds drift with occasional rain
───────────────────────────────────── <- sky
           |  │  |                   <- raindrops (when cloud overhead)
═════════════════════════════════════ <- ground row
 Plant1  Plant2  Plant3 ...          <- 6-10 plants in columns
```

## Plants

6 varieties, randomly assigned at spawn. Each has 6 growth stages rendered upward from the ground row using `canvas.set_char()`.

| Stage | Description |
|-------|-------------|
| 0 | `.` — seed |
| 1 | `,` — sprout |
| 2 | `\|` short stem + leaf `~` |
| 3 | taller stem with branching chars (`Y`, `/\`) |
| 4 | flower bud `o` or `*` on top |
| 5 | full bloom: `(@)`, `{a}`, `**`, `ö`, etc — colored head |

- Stem color: green
- Flower head colors: orange, blue, white, yellow, magenta (per variety)
- Stage 5 is the maximum — plants do not grow beyond it

## Rain Mechanic

- 1–3 clouds drift across the screen (`(~~~)` shape, varying width)
- Each cloud has a `raining: bool` that randomly activates (~10% chance/sec) and lasts 3–8 seconds, then enters a cooldown before it can rain again
- While raining: drops spawn from cloud's x-span at ~2 drops/second/column
- `Raindrop`: tracks x, y, fall speed
- On reaching ground row: check if any plant's column range overlaps drop x → if `plant.stage < 5`, increment stage
- Splash effect: 2–3 `·` chars at impact point, despawn after 0.3s

## Sun

- Single glyph `(*)` or `☀` moving slowly left→right, wraps around
- Bright yellow color
- Pure atmosphere — no gameplay effect

## Data Structures

```rust
struct Plant { col: usize, variety: usize, stage: usize }
struct Raindrop { x: f64, y: f64, speed: f64 }
struct Cloud { x: f64, width: usize, raining: bool, rain_timer: f64, rain_cooldown: f64 }
struct Splash { x: usize, y: usize, ttl: f64 }

pub struct Garden {
    plants: Vec<Plant>,
    clouds: Vec<Cloud>,
    drops: Vec<Raindrop>,
    splashes: Vec<Splash>,
    sun_x: f64,
    width: usize,
    height: usize,
}
```

## Render Mode

`preferred_render()` returns `RenderMode::Ascii` — all drawing via `canvas.set_char()`.

## Integration Points

- New file: `src/animations/garden.rs`
- `src/animations/mod.rs`: add `pub mod garden`, entry in `ANIMATIONS`, `ANIMATION_NAMES`, and `create()` match arm
