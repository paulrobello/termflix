# ENH-005 — Named per-animation external parameters

## Goal

External control can set animation-specific parameters by name (`{"params": {"cohesion": 0.6}}`), normalized to 0..1 and validated against each animation's declared parameter list. The global `speed`/`intensity`/`color_shift` keep their global meaning and stop silently doubling as per-animation knobs with incompatible ranges. `supported_params()` gets a real runtime caller, and `--list-params <anim>` prints what each animation accepts.

## Current state

- `src/external.rs:2-10` `ExternalParams` has fixed fields `animation, speed, intensity, color_shift, scale, render, color`.
- 18 animations implement `set_params`/`supported_params`, and several reinterpret the global fields with conflicting ranges (DOC-007):
  - `boids.rs:116-127`: `intensity` → cohesion clamped 0.001..0.05, so the neutral 1.0 pins it to maximum. `color_shift` → separation clamped 0.5..5.0.
  - `snake.rs:154-155`: `speed` → move interval 0.02..0.2 (higher means slower).
  - `sort.rs:220`: `speed` → ops per frame 1..20.
  - `particles.rs:75`: `color_shift` → drag, where the neutral 0 pins it to 0.9.
  - wave: amplitude.
- The same fields also drive the global time multiplier (`src/main.rs:~917`, `speed.clamp(0.1, 5.0)`) and the global brightness/hue (`apply_effects`).
- `supported_params` is `#[allow(dead_code)]` at `src/animations/mod.rs:86-90`. It is only read in tests.
- `CurrentState` duplicates every field into a `params` mirror (ARC-013).
- Audit context: DOC-007, ARC-013.

## Design

- Declare parameters with metadata:
  ```rust
  pub struct ParamSpec { pub name: &'static str, pub min: f64, pub max: f64, pub default: f64, pub help: &'static str }
  fn param_specs(&self) -> &'static [ParamSpec] { &[] }
  fn set_param(&mut self, name: &str, value01: f64) {}
  ```
  `value01` is normalized 0..1, and the animation maps it with `min + v * (max - min)`, so every external value has one shared, documented range.
- `ExternalParams` gains `#[serde(default)] params: BTreeMap<String, f64>`. `CurrentState` stores pending named params, and `run_loop` applies each one via `set_param` when the name is in `param_specs()`. Unknown names produce a one-time status-bar note (via the ARC-013 `last_error` mechanism if it has landed, otherwise they are dropped).
- **Migration of existing overloads.** Each of the 18 `set_params` impls becomes `param_specs` + `set_param` with a descriptive name (`cohesion`, `separation`, `move_interval`, `ops_per_frame`, `drag`, `amplitude`, …).
- **Compatibility of the global fields.** The old behavior where globals feed per-animation knobs is kept for one release behind a deprecation. Keep `set_params` calling the old mapping, but only when no named params were ever received. Log nothing, and document it as deprecated in `docs/EXTERNAL_ANIMATION.md`. Remove it in the next minor version.
- `supported_params` is replaced by `param_specs`, and the `dead_code` allow goes away.
- CLI: `--list-params [anim]` prints the specs (all animations, or one).

## Steps

1. Inventory: `grep -n "fn set_params" -A15 src/animations/*.rs` to tabulate every animation, which global field it reads, its internal target and its clamp range. Put the table in the PR description. It is also the DOC-007 table.
2. Add `ParamSpec`, `param_specs`, and `set_param` with defaults to the trait. Add the `params` map to `ExternalParams` (the serde default keeps old JSON valid) and to `CurrentState`, with unit tests for merging named params.
3. In `run_loop`, after `set_params`, drain pending named params and call `set_param` for names in `param_specs()`.
4. Migrate the 18 animations one at a time. Each gets a `param_specs` list and `set_param`. Keep its existing `set_params` as the deprecated fallback, guarded by a `named_params_seen: bool` field.
5. Delete `supported_params` from the trait and all impls, and update the registry test in `src/animations/mod.rs` to test `param_specs` (every spec satisfies `min < max` and `default` in `[min, max]`, and names are unique per animation).
6. Add `--list-params` to `Cli` and handle it next to `--list`.
7. Docs: rewrite the per-animation section of `docs/EXTERNAL_ANIMATION.md` from `--list-params` output, add a `params` example, and mark the global-field overloads deprecated. Update the README and CHANGELOG.

## Files to touch

`src/animations/mod.rs`, `src/external.rs`, `src/main.rs`, the 18 animation files that implement `set_params` (`grep -ln "fn set_params" src/animations`), `docs/EXTERNAL_ANIMATION.md`, `README.md`, `CHANGELOG.md`.

Sequencing: after ARC-013 (the external-state single copy) and ARC-001 (the `run_loop` decomposition), because both rewrite the code this plan extends.

## Verify

- `cargo test external` passes, including a test that `{"params":{"cohesion":0.5}}` merges into `CurrentState`
- `cargo test param_specs_are_valid` passes for all animations
- `echo '{"params":{"cohesion":1.0}}' | ./target/release/termflix boids` runs, and boids visibly clump (manual)
- `echo '{"intensity":1.0}' | ./target/release/termflix boids` still behaves as today (deprecated path)
- `./target/release/termflix --list-params boids` lists `cohesion` and `separation` with ranges
- `grep -rn "supported_params" src | wc -l` prints `0`
- `make checkall` exits 0

## Rollback

Named params are additive and serde-defaulted, so old control scripts keep working throughout. Reverting removes the `params` key support. Scripts that adopted named params would then have those keys ignored silently (serde ignores unknown fields by default; confirm `ExternalParams` has no `deny_unknown_fields`).
