# ENH-001 — Seeded, injectable RNG (`--seed`)

## Goal

Every animation draws randomness from one seedable RNG so that `--seed N` (and the gallery, and tests) produce byte-identical frames run to run. This enables golden-frame tests (ENH-004), reproducible gallery diffs, and bug reports that say "seed 42, frame 300".

## Current state

- 44 files make 110 calls to `rand::rng()` or store `rand::rngs::ThreadRng` (`grep -rn "rand::rng()\|ThreadRng" src`). Examples: `src/animations/snake.rs:51,61,77`, `sort.rs:41,53,65`, `snow.rs:20,25,43`, `nbody.rs:47,81` (fetched per spawn), `src/generators/mod.rs:122,171` (`ParticleSystem`).
- The constructor contract is `fn new(width: usize, height: usize, scale: f64) -> Self`, called only through `declare_animations!` in `src/animations/mod.rs:93-110`.
- `rand = "0.10"` (Cargo.lock resolves 0.10.2). It includes `rand::rngs::StdRng` (ChaCha12, `SeedableRng`) under default features. `SmallRng` needs the `small_rng` feature — use `StdRng` to avoid a Cargo feature change.
- Audit context: ARC-020 (AUDIT.md) identified this; QA-004's size sweep and the gallery would both benefit.

## Design

- New module `src/rng.rs`:
  ```rust
  //! Seedable RNG source for animations (thread-local).
  use rand::{SeedableRng, rngs::StdRng};
  use std::cell::Cell;

  thread_local! {
      /// `Some((seed, next_stream))` when seeded on this thread.
      static STATE: Cell<Option<(u64, u64)>> = const { Cell::new(None) };
  }

  /// Seed this thread's RNG source. Call at startup and before each gallery capture / test case.
  pub fn set_seed(seed: u64) { STATE.with(|s| s.set(Some((seed, 0)))); }

  /// A new RNG: a deterministic stream per call when seeded, OS entropy otherwise.
  pub fn new_rng() -> StdRng {
      STATE.with(|s| match s.get() {
          Some((seed, n)) => {
              s.set(Some((seed, n + 1)));
              StdRng::seed_from_u64(seed ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15))
          }
          None => rand::make_rng(), // verify the rand 0.10 name: `StdRng::from_os_rng()` or `rand::make_rng::<StdRng>()`
      })
  }
  ```
  **Thread-local, not a global atomic.** `cargo test` runs tests on parallel threads. With a process-wide seed and counter, one test's `set_seed` or construction would perturb another's stream and the determinism/golden tests would flake. Animations are constructed on the thread that calls `set_seed` (the main loop, the gallery, and each test), so thread-local state is deterministic everywhere. Threading a seed through `new(width, height, scale)` instead would change 60 constructors and the macro. The per-call stream counter makes each `new_rng()` distinct but reproducible, which requires deterministic construction order (true, since construction is single-threaded per caller).
- Every animation stores `rng: StdRng` created by `crate::rng::new_rng()` in `new`, and never calls `rand::rng()` again. Per-call `rand::rng()` sites (nbody `spawn_body`, generators) use `self.rng`.
- `ParticleSystem` (`src/generators/mod.rs`) gets its own `rng: StdRng` field.
- CLI: `--seed <u64>` in `Cli` (`src/main.rs:37-175`), config key `seed` (`src/config.rs`), plus `--gallery` always seeds (default seed `1`, or `--seed`), calling `rng::set_seed` before each `capture_animation`.
- Any use of time-based randomness (grep `SystemTime` / `Instant` used as entropy in animations) must also go through `new_rng()`.

## Steps

1. Read the rand 0.10.2 API in `~/.cargo/registry/src/*/rand-0.10.2/src/rngs/` to confirm the OS-seeded constructor name and that `StdRng` + `SeedableRng::seed_from_u64` exist without extra features. Adjust `src/rng.rs` accordingly.
2. Add `src/rng.rs` and `mod rng;` in `src/main.rs`. Unit tests: two `set_seed(7)` sequences give equal `random::<u64>()` outputs, and different seeds differ.
3. Convert `src/generators/mod.rs` first (it is shared), then the animations in batches of ~10 files. Mechanical pattern per file:
   - `rng: rand::rngs::ThreadRng` → `rng: rand::rngs::StdRng`
   - `rand::rng()` in `new` → `crate::rng::new_rng()`
   - `rand::rng()` in methods → `self.rng` (add the field if missing, and fix borrow conflicts by copying values out before `self.rng.random...` calls)
   - `fn random_glyph(rng: &mut rand::rngs::ThreadRng)` style helpers → `&mut impl rand::Rng` (verify the trait name in 0.10: `rand::Rng` or `rand::RngExt`)
4. After each batch: `grep -rn "rand::rng()\|ThreadRng" src/animations | wc -l` decreases; run `make checkall`.
5. Add `--seed` to `Cli`, `seed: Option<u64>` to config, and call `rng::set_seed` in `main` before `run_loop` when set. When transitioning animations in `run_loop`, do not reseed (so a seeded live run is reproducible only from start, which is fine).
6. Gallery: in `run_gallery`, call `rng::set_seed(seed)` immediately before each `capture_animation` so each animation's capture is independent of order and filter.
7. Determinism test in `src/animations/mod.rs` tests: for every name, `set_seed(42)`, create at 40x12 HalfBlock, run 20 updates with fixed dt/time, and hash `canvas.pixels` (bit patterns via `to_bits`) plus `canvas.colors`. Repeat and assert the hashes are equal. Report all non-deterministic animations at once.
8. Fix any animation the test flags (usually a leftover `rand::rng()` or a `HashMap` iteration-order dependency — use `BTreeMap` or a sorted Vec).
9. Document `--seed` in README (CLI reference) and CHANGELOG `[Unreleased]`.

## Files to touch

`src/rng.rs` (new), `src/main.rs`, `src/config.rs`, `src/gallery.rs`, `src/generators/mod.rs`, the 44 files from `grep -rln "rand::rng()\|ThreadRng" src`, `src/animations/mod.rs` (test), `README.md`, `CHANGELOG.md`.

Conflicts: land after ARC-012 (the `hsv_to_rgb` hoist) and preferably after QA-008 to avoid per-animation merge churn. Independent of ARC-001 except for the `Cli`/`main` edit.

## Verify

- `grep -rn "rand::rng()\|ThreadRng" src | wc -l` prints `0`
- `cargo test rng` passes (seed-equality unit tests)
- `cargo test every_animation_is_deterministic_with_seed` passes for all 60 animations
- `./target/release/termflix --gallery fire,plasma,matrix --seed 42 --gallery-dir /tmp/a && ./target/release/termflix --gallery fire,plasma,matrix --seed 42 --gallery-dir /tmp/b && cmp /tmp/a/fire.png /tmp/b/fire.png` exits 0 (the same for `.gif`)
- `make checkall` exits 0

## Rollback

Pure additive plus mechanical RNG substitution. Revert the commit series. Unseeded behavior uses OS entropy as today, so users see no change unless they pass `--seed`.
