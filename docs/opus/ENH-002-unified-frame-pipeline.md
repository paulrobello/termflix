# ENH-002 — One frame pipeline shared by live, gallery and benchmarks

## Goal

A single function produces a finished frame from an animation (clear → update → smoothing → effects → color assist → post-process). `run_loop`, the gallery and the encoder benchmark all call it, so gallery output matches live output and a new post-effect is added in one place. The `Animation` trait documents that the canvas arrives cleared.

## Current state

- Live path, `src/main.rs:916-987` (inside `run_loop`): `set_params` → `update` (no clear) → `apply_smoothing` (if tau > 0) → transition factor → `apply_effects(intensity, hue)` → `apply_color_assist` → `post_process`.
- Gallery path, `src/gallery.rs:108-113`: `canvas.clear()` → `update` → `apply_effects(1.0, 0.0)` → `post_process` (bloom hardcoded 0.4). It skips smoothing and color assist.
- Encoder benchmark: `bench_dirty` in `src/render/encoder.rs` (around line 590) hand-rolls its own loop.
- 57 of 60 animations call `canvas.clear()` inside `update`. `plasma`, `wave` and `spiral` overwrite every pixel instead. The trait (`src/animations/mod.rs:64-91`) does not say who clears.
- Audit context: ARC-007.

## Design

- New `src/render/pipeline.rs`:
  ```rust
  /// Per-frame effect settings applied after the animation draws.
  pub struct FrameEffects<'a> {
      pub smoothing_alpha: Option<f64>, // None = off
      pub intensity: f64,               // includes transition fade factor
      pub hue_shift: f64,
      pub assist: &'a ColorAssist,
      pub postproc: &'a PostProcessConfig,
  }

  /// Clear, update, and post-process one frame. Returns time spent in `update`.
  pub fn produce_frame(anim: &mut dyn Animation, canvas: &mut Canvas, dt: f64, t: f64, fx: &FrameEffects) -> Duration
  ```
  Order inside: `canvas.clear()` → timed `anim.update` → `apply_smoothing(alpha)` if Some → `apply_effects` → `apply_color_assist` → `post_process`.
- **Smoothing and clear interaction.** `apply_smoothing` blends against `canvas.prev_pixels`, which is a separate buffer (`src/render/canvas.rs:64,195-212`), so clearing `pixels` first does not break smoothing. Confirm by reading `apply_smoothing` before implementing.
- **Clearing cost.** Clearing cost for plasma/wave/spiral is one `fill` per frame, which is negligible, and removing their full overwrite is not required.
- Remove `canvas.clear()` from the 57 `update` impls afterwards (a separate commit), and document on the trait: "The canvas is cleared before `update` is called."

## Steps

1. Read `Canvas::apply_smoothing`, `apply_effects`, `apply_color_assist` and `post_process` in `src/render/canvas.rs` to confirm the signatures and that their order matches the live path.
2. Add `src/render/pipeline.rs`, export it from `src/render/mod.rs`, and give it a unit test: an animation stub that sets one pixel produces the same canvas as the hand-rolled live sequence.
3. Switch `run_loop` to `produce_frame`, keeping `set_params` before it and the transition-factor computation feeding `fx.intensity`. The profiler's `update_dur` comes from the return value.
4. Switch `gallery::capture_animation` to `produce_frame` with `FrameEffects { smoothing_alpha: None, intensity: 1.0, hue_shift: 0.0, assist: &ColorAssist::None, postproc: &postproc }`. This is behavior-identical to today apart from the pipeline being shared. Optionally expose `--palette` for gallery later (out of scope).
5. Switch `bench_dirty` to `produce_frame`.
6. Separate commit: remove `canvas.clear()` from animation `update` bodies (`grep -ln "canvas.clear()" src/animations`). Double clearing is harmless, so this step is safe to defer or skip.
7. Update the trait doc in `src/animations/mod.rs`, the ARCHITECTURE.md pipeline section, and CLAUDE.md's pipeline line.

## Files to touch

`src/render/pipeline.rs` (new), `src/render/mod.rs`, `src/main.rs`, `src/gallery.rs`, `src/render/encoder.rs`, `src/animations/mod.rs`, optionally 57 animation files (step 6), `docs/ARCHITECTURE.md`, `CLAUDE.md`.

Sequencing: after ARC-001 (the `run_loop` decomposition), because it touches the same region. If ARC-001 has not landed, do steps 1–5 against the current `run_loop` in place.

## Verify

- `cargo test pipeline` passes (the stub-animation equivalence test)
- `grep -n "apply_effects\|post_process" src/main.rs src/gallery.rs src/render/encoder.rs` shows matches only inside `src/render/pipeline.rs` callers, and no direct calls remain in those three files
- `cargo test every_animation_survives_all_sizes` passes (QA-004)
- `make gallery ARGS="fire,plasma,wave,spiral"` completes and the four PNGs are visually unchanged
- `make checkall` exits 0

## Rollback

Revert the commits. Step 6 is independent and can be reverted alone. No data formats or CLI change.
