# ENH-004 — Golden-frame regression tests for every animation

## Goal

A test renders each animation deterministically (via ENH-001's seed) at a fixed size and compares a hash of the frame at a fixed time against a checked-in golden value. Any unintended visual change (a refactor like QA-008, the `hsv_to_rgb` hoist in ARC-012, or a pipeline change in ENH-002) fails the test and names the animation. An env var regenerates the goldens intentionally.

## Current state

- 59 of 60 animations have no behavioral test (QA-004). `matrix.rs:297` has the only per-animation test.
- `src/render/encoder.rs` already has snapshot-style tests with an ignored regenerator (per the code-quality audit: "2 ignored — a snapshot regenerator and a benchmark"). Reuse its pattern (read the file to find the regenerator test and how it stores snapshots).
- Randomness is unseeded until ENH-001 lands, which blocks this plan.

## Design

- Test file `src/animations/golden.rs`, included as `#[cfg(test)] mod golden;` from `src/animations/mod.rs`.
- For each name in `ANIMATION_NAMES`: `crate::rng::set_seed(1234)`, build a 40x12 canvas in the animation's preferred render mode (via the registry or by constructing and asking), then run `produce_frame` (ENH-002; otherwise the manual sequence) for 48 frames at dt = 1/24 with no smoothing, no assist, and default postproc. Hash the final canvas as FNV-1a 64 over `pixels[i].to_bits()` and `colors[i]`, and hash `build_grid()` cell chars and colors as well.
- Goldens in `src/animations/golden_hashes.txt`, one line per animation: `name pixel_hash grid_hash`. The test loads it with `include_str!`.
- Regenerate with `TERMFLIX_UPDATE_GOLDEN=1 cargo test golden`, which rewrites the file (through `env!("CARGO_MANIFEST_DIR")`) instead of asserting.
- Floating-point determinism: hashes of `f64` bits are stable on one platform but may differ across architectures (FMA contraction on aarch64 vs x86_64). Rust does not auto-fuse FMA, but `libm` functions (`sin`, `exp`) can differ across platforms. To keep CI stable, quantize before hashing: `(p.clamp(0.0, 4.0) * 1024.0).round() as u32`. Also hash only the `build_grid()` output, which is already quantized to chars and 8-bit colors. If a mismatch persists across OSes, gate the test with `#[cfg(all(target_os = "macos", target_arch = "aarch64"))]` plus a note, and run it in the macOS CI job.

## Steps

1. Confirm that ENH-001 (the `crate::rng` module and `set_seed`) has landed and that its state is **thread-local**, as the plan specifies. If it has not landed, stop: this plan depends on it. The golden test calls `set_seed` per animation on the test thread. A process-global seed would make it flaky under `cargo test`'s parallel threads.
2. Read the encoder snapshot-regeneration pattern in `src/render/encoder.rs` and mirror it.
3. Write `golden.rs` with the harness and the regeneration mode. Generate the initial `golden_hashes.txt` and commit it with the test.
4. Run the test three times locally to confirm stability. Run it under `--release` as well, since the float code must match between debug and release. If it does not match, quantize harder.
5. If Linux and Windows CI are available (ci.yml test matrix), run it there. Apply the platform gate from Design only if needed.
6. Document the regeneration command in CONTRIBUTING.md (DOC-014) and CLAUDE.md: "intentional visual change → regenerate goldens and review the diff of `golden_hashes.txt`".

## Files to touch

`src/animations/golden.rs` (new), `src/animations/golden_hashes.txt` (new), `src/animations/mod.rs`, `CLAUDE.md`, `CONTRIBUTING.md` (if it exists).

Depends on: ENH-001 (hard). ENH-002 (soft; it simplifies the harness).

## Verify

- `cargo test golden` passes, and three consecutive runs give identical results
- `cargo test --release golden` passes
- Changing one constant in `src/animations/plasma.rs` (temporarily, not committed) makes `cargo test golden` fail and name `plasma`, and reverting the change makes it pass again
- `TERMFLIX_UPDATE_GOLDEN=1 cargo test golden` rewrites `src/animations/golden_hashes.txt` with 60 lines
- `make checkall` exits 0

## Rollback

Delete the two new files and the `mod golden;` line. The test adds no production code.
