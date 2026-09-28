# Audit Remediation Playbook — termflix

> Companion to `AUDIT.md` (2026-09-27, commit `ef5c840`). `/fix-audit` reads this file and hands each
> phase agent its own entries. Entries follow the `## Remediation Plan` phase order in AUDIT.md.
> Absorbed IDs (see AUDIT.md "Deduplication Map") have no entry of their own; they are closed by
> their surviving ID.
>
> **Global rules for every agent**
> - Work in a git worktree. Re-read every file before editing: line numbers below are from `ef5c840`
>   and drift as earlier phases land. Locate symbols with parsight (`find_symbol`,
>   `get_symbol_context`, repository_id `termflix`) rather than trusting line numbers.
> - Gate: after ARC-003 lands, `make checkall` (= fmt-check + clippy --all-targets --all-features
>   + test + build). Before ARC-003 lands, run
>   `cargo fmt -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test && cargo build`.
> - Never edit `Cargo.lock` by hand. Do not bump dependencies.
> - Kanban cards exist for Critical/High issues (tag `audit-2026-09-27`); the title prefix is the ID.

---

## Phase 1 — Security (promoted, sequential)

### [SEC-001] Bound recording dimensions in GIF export
- **Files**: `src/main.rs` (`detect_recording_size`, ~1171-1201; call site ~240-243), `src/gif.rs` (`VirtualTerminal::new` ~44-46, cell indexing ~144-146, `export_gif` ~494-503, `export_gif_pixels` ~670-676)
- **Steps**:
  1. In `src/gif.rs` add `pub const MAX_GIF_COLS: usize = 1000;` and `pub const MAX_GIF_ROWS: usize = 500;` near the top.
  2. In `detect_recording_size`, return `(max_col.min(MAX_GIF_COLS), max_row.min(MAX_GIF_ROWS))`. Parse with `parse::<usize>()` as today; the clamp handles oversized values. A `parse` failure on a 20-digit number is already ignored.
  3. At the top of `export_gif` (and `export_gif_pixels`, for its pixel width/height), validate:
     ```rust
     if term_cols == 0 || term_rows == 0 || term_cols > MAX_GIF_COLS || term_rows > MAX_GIF_ROWS {
         return Err(io::Error::new(io::ErrorKind::InvalidInput, format!(
             "recording size {term_cols}x{term_rows} outside 1..={MAX_GIF_COLS}x1..={MAX_GIF_ROWS}")));
     }
     ```
     Then derive pixel dimensions with `u16::try_from(px_w).map_err(|_| io::Error::new(InvalidInput, ...))?` instead of `as u16`. For `export_gif_pixels`, pick a pixel ceiling of `u16::MAX` on each axis plus `checked_mul` for the buffer length.
  4. In `VirtualTerminal`, clamp the cursor to `cols-1`/`rows-1` on CUP (it likely already does for normal input; verify) and bounds-check the cell index with `if let Some(cell) = self.cells.get_mut(idx)` instead of direct indexing.
  5. Add tests in `src/gif.rs` `#[cfg(test)]`: (a) `detect_recording_size` on a frame containing `\x1b[70000;70000H` returns `(1000, 500)`; (b) `export_gif` with `term_cols = usize::MAX` returns `ErrorKind::InvalidInput`; (c) a frame with `\x1b[4294967296;4294967296H` exports without panic.
- **Method**: The root cause is trusting file content for allocation size. Clamping at detection is the policy; validation in `export_gif` is the defence for any other caller (it is `pub`). Do not move `detect_recording_size` out of `main.rs` here — ARC-023/ARC-001 own structure. `detect_recording_size` is only called from `main`; confirm with `get_symbol_context`.
- **Verify**:
  - `cargo test gif`
  - Repro no longer OOMs: build a file with `printf 'ASCIIANIM v1\nFRAMES 1\n---\nT 0\n%s\n' "$(printf '\033[70000;70000H' | base64)" > $SCRATCH/big.asciianim && /usr/bin/time -l ./target/release/termflix --play $SCRATCH/big.asciianim --export-gif $SCRATCH/o.gif` completes with max RSS < 200 MB (macOS `time -l` reports bytes).
  - The `4294967296` variant exits 0 or with an error message, never exit 101.

### [SEC-003] Filter terminal escape sequences on `--play`
- **Files**: `src/record.rs` (`Player::load` ~113-149, `Player::play` ~191-193)
- **Steps**:
  1. Add `fn sanitize_frame(content: &str) -> String` in `src/record.rs`. Walk bytes/chars:
     - Keep printable chars (including non-ASCII, which termflix emits for braille/blocks/kana), `\n`, `\r`.
     - On `ESC [`: read parameter bytes `0x30..=0x3F`, intermediate `0x20..=0x2F`, final `0x40..=0x7E`. Keep the whole sequence only if final ∈ {`H`, `m`, `K`, `J`} or it is `ESC [ ? <digits> h|l` with digits in {25, 2026}. Otherwise drop the whole sequence.
     - On `ESC ]`, `ESC P`, `ESC _`, `ESC ^`, `ESC X`: drop through the terminator (BEL `0x07` or `ESC \`), or to end of input.
     - Drop any other `ESC x` pair and any other C0 (except `\n`, `\r`) and C1 (`U+0080..=U+009F`) control.
  2. Apply in `Player::load` when building each `Frame` (so `--export-gif` also sees sanitized content).
  3. Tests: OSC 52 (`\x1b]52;c;ZXZpbA==\x07`) removed; `\x1b[38;2;1;2;3m` kept; `\x1b[?2026h` kept; `\x1b[6n` (DSR) removed; braille text preserved byte-for-byte.
- **Method**: Allowlist, not denylist. Use `src/gif.rs` `VirtualTerminal::process` as the reference for what termflix emits; run `grep -o '\\x1b\[[^a-zA-Z]*[a-zA-Z]' src/render/encoder.rs`-style inspection of the encoder to confirm the final bytes it uses before finalizing the allowlist. If the encoder emits a final byte not listed (e.g. `G` or `X`), add it and note it in a comment.
- **Verify**: `cargo test record`; record a real session (`./target/release/termflix fire --record $SCRATCH/r.asciianim`, quit after 2 s) and play it back — visually unchanged.

### [SEC-004] Make playback wait interruptible and reject absurd timestamps
- **Files**: `src/record.rs` (`load` ~125-128, `play` ~174-178)
- **Steps**:
  1. In `load`, after parsing `timestamp_ms`: reject if `timestamp_ms > 24 * 3600 * 1000` or less than the previous frame's timestamp → `InvalidData` "timestamp out of order or too large".
  2. In `play`, replace `std::thread::sleep(target - elapsed)` + zero-timeout poll with a loop: `while start.elapsed() < target { let wait = target - start.elapsed(); if event::poll(wait.min(Duration::from_millis(50)))? { if quit key → break 'frames; } }`. Label the outer `for` loop `'frames`.
  3. Also treat `Ctrl+C` (KeyCode::Char('c') + CONTROL) as quit, matching `run_loop`.
- **Method**: This finding was established by code reading, not a TTY reproduction. The poll-based wait fixes it regardless. Keep ARC-010's future keybinding integration in mind: isolate the quit check in `fn is_quit(&KeyEvent) -> bool`.
- **Verify**: `cargo test record` with tests for decreasing and oversized timestamps; manual: a file with a 60 s gap quits immediately on `q`.

### [SEC-007] Do not echo raw file content in parse errors
- **Files**: `src/record.rs` (~100-101)
- **Steps**: Replace `format!("Invalid header: {}", header)` with `format!("Invalid header: {}", header.chars().take(40).collect::<String>().escape_debug())`.
- **Verify**: `cargo test record` (add a test asserting the error string contains no `\x1b`).

### [SEC-008] Bound record/playback memory
- **Files**: `src/record.rs` (`Recorder` ~30-35, `load` ~108-149)
- **Steps**:
  1. Constants: `MAX_FRAMES: usize = 100_000`, `MAX_FRAME_BYTES: usize = 4 * 1024 * 1024`.
  2. In `load`: use the parsed `FRAMES` count (rename `_frame_count` → `frame_count`) to reject values > `MAX_FRAMES`; reject when `frames.len()` exceeds `MAX_FRAMES`; reject an encoded line longer than `MAX_FRAME_BYTES * 4 / 3 + 4` before decoding.
  3. In `Recorder`, stop pushing (and set a `truncated` flag reported on save) after `MAX_FRAMES`.
- **Verify**: `cargo test record` with a synthetic oversized header.

---

## Phase 2 — Gates, size safety, dead code, color hoist, `run_loop` (sequential, in this order)

### [ARC-003] Install and align the quality gates (absorbs ARC-004, DOC-008 hook claim)
- **Files**: `Makefile`, `.githooks/pre-commit`, `.github/workflows/ci.yml`, `CLAUDE.md`
- **Steps**:
  1. `Makefile`: `lint:` → `cargo clippy --all-targets --all-features -- -D warnings`. `checkall: fmt-check lint test build` (drop `typecheck` — `build` covers it; keep the `typecheck` target itself). Add `hooks:` → `git config core.hooksPath .githooks`. Add `hooks` and `fmt-check` to `.PHONY`.
  2. `.githooks/pre-commit`: replace the body after `set -e` with `exec make checkall`. Keep it executable (`chmod +x` is already set; verify with `ls -l`).
  3. `ci.yml`: `on:` add `push: { branches: [main] }` and `pull_request:` alongside `workflow_dispatch`.
  4. `CLAUDE.md`: in Build & Development, say the hook is enabled with `make hooks` (one-time per clone) and that `make lint` matches CI.
  5. Run `make hooks` in the worktree only if the user wants it active there; it is local config, not committed.
- **Method**: Fixing clippy on `--all-targets` may surface existing warnings in test code. Fix them in this same change (they are the gate's backlog). If more than ~20, stop and report.
- **Verify**: `make checkall` exits 0 (`make checkall; echo EXIT=$?`); `git -c core.hooksPath=.githooks commit --dry-run` style check is not possible — instead run `.githooks/pre-commit; echo EXIT=$?`.

### [QA-004] Animation size-sweep test (write first; it fails until QA-001/QA-002 land)

> **QA-004, QA-001 and QA-002 are one commit.** Write this test first and use it to drive the two
> fixes, but commit all three together — never commit the test failing (the ARC-003 hook runs
> `make checkall` and would reject it). If a split commit is required, commit the test with
> `#[ignore = "enabled by QA-001/QA-002"]` and remove the attribute in the fix commit.
- **Files**: `src/animations/mod.rs` (`#[cfg(test)] mod tests`, ~190-287)
- **Steps**:
  1. Add:
     ```rust
     #[test]
     fn every_animation_survives_all_sizes() {
         use crate::render::{Canvas, ColorMode, RenderMode};
         let sizes = [(1, 1), (2, 2), (10, 4), (10, 5), (10, 8), (80, 24), (300, 100)];
         let modes = [RenderMode::Braille, RenderMode::HalfBlock, RenderMode::Ascii];
         for &name in ANIMATION_NAMES {
             for &mode in &modes {
                 for &(c, r) in &sizes {
                     let mut canvas = Canvas::new(c, r, mode, ColorMode::TrueColor);
                     let mut anim = create(name, canvas.width, canvas.height, 1.0).unwrap();
                     anim.on_resize(canvas.width, canvas.height);
                     for i in 0..30 { anim.update(&mut canvas, 1.0 / 24.0, i as f64 / 24.0); }
                     assert!(canvas.pixels.iter().all(|p| p.is_finite()), "{name} {mode:?} {c}x{r}");
                 }
             }
         }
     }
     ```
     Adjust imports to the actual module paths (`crate::render::canvas::...`). Wrap each case in `std::panic::catch_unwind` collecting failures into a Vec so one run reports every failing (name, mode, size) — much more useful than stopping at the first.
  2. Add a shrink test: create at 80x24, `on_resize` to 2x2 on the same instance, 5 updates.
  3. If the sweep is slow (>10 s), cap `(300,100)` to HalfBlock only.
- **Method**: Sizes are terminal cells, so the canvas `Canvas::new` produces the real pixel dims. If QA-002 chooses to clamp inside `Canvas::new`, the 1x1/2x2 cases then test the clamp. Pixels need not be in [0,1] (some animations write >1 before post-processing); assert finiteness only.
- **Verify**: `cargo test every_animation_survives_all_sizes` — expected to FAIL now listing at least matrix, nbody, flappy_bird, aurora, cells, invaders, pong, langton; must PASS after QA-001 + QA-002.

### [QA-001] Non-empty random ranges in matrix, nbody, flappy_bird
- **Files**: `src/animations/matrix.rs` (`create_drops` ~24-43, layer lengths ~145-147), `src/animations/nbody.rs` (`spawn_body` ~80-83, also `spawn_initial_bodies` ~46), `src/animations/flappy_bird.rs` (`tune_params` ~62-72, `spawn_pipe` ~85-89)
- **Steps**:
  1. matrix: in `new`, compute `let cap = (height / 2).max(4);` and use `(3, 5.min(cap).max(4))`, `(5, 8.min(cap).max(6))`, `(8, 12.min(cap).max(9))` so every `(min, max)` has `max > min`. Also guard `width == 0` in `create_drops` (`random_range(0..width)` panics on 0): return an empty Vec.
  2. nbody: `let max_dist = (self.width.min(self.height) as f64 * 0.4).max(5.0 + 1.0); let dist = rng.random_range(5.0..max_dist);`. Apply the same pattern to every `random_range` in the file (grep `random_range` in nbody.rs).
  3. flappy_bird: in `spawn_pipe`, `let lo = margin; let hi = (h - margin).max(lo + 1.0); let gap_center = self.rng.random_range(lo..hi);`. Also cap `gap_size` so it never exceeds `h * 0.8` in `tune_params`.
- **Method**: rand 0.10 `random_range` panics on empty ranges for both ints and floats. Behavior at normal sizes must be unchanged: the `.max(...)` only bites at tiny heights. Check other `random_range` calls in these three files with size-derived bounds.
- **Verify**: `cargo test every_animation_survives_all_sizes` shows none of the three; the gallery repro `./target/release/termflix --gallery matrix --gallery-dir $SCRATCH/g --gallery-cols 10 --gallery-rows 5 --gallery-wait 0.5 --gallery-duration 0.5; echo EXIT=$?` → 0 for matrix, nbody, flappy_bird.

### [QA-002] Central minimum-size policy plus per-animation hardening
- **Files**: `src/render/canvas.rs` (`Canvas::new` ~68-92), `src/main.rs` (startup ~621-648, rebuild ~821, transition ~885, switch ~945), `src/animations/aurora.rs` (~34-35), `cells.rs`, `invaders.rs`, `pong.rs` (`f64::clamp` sites), `langton.rs` (~74-75)
- **Steps**:
  1. Central clamp in `Canvas::new`: `let term_cols = term_cols.max(MIN_TERM_COLS); let term_rows = term_rows.max(MIN_TERM_ROWS);` with `pub const MIN_TERM_COLS: usize = 10; pub const MIN_TERM_ROWS: usize = 4;` (4 = the 5-row floor minus the status row). A canvas larger than the terminal is safe: the encoder writes cells beyond the visible area which the terminal clips — **verify** this by checking whether the encoder emits CUP beyond `rows`; if it could scroll the screen, instead keep a real-size canvas and render a "terminal too small" line from `run_loop` when below the floor, skipping `update`.
  2. `src/main.rs`: make the rebuild guard use the same constants (`cur_cols as usize >= MIN_TERM_COLS && cur_rows as usize >= MIN_TERM_ROWS + 1`).
  3. Harden animations regardless of the clamp:
     - aurora: `% (canvas.height as u64 / 3).max(1)` and `% (canvas.width as u64).max(1)`.
     - cells/invaders/pong: replace `x.clamp(a, b)` where `a` may exceed `b` with `x.clamp(a.min(b), b.max(a))` or `x.max(a).min(b)` (never panics). Find sites: `grep -n "\.clamp(" src/animations/{cells,invaders,pong}.rs`.
     - langton: `random_range(width as i32 / 3..(width as i32 * 2 / 3).max(width as i32 / 3 + 1))`, same for y.
- **Method**: Defence in depth: the test in QA-004 uses `Canvas::new` so it covers the clamp; the per-animation hardening protects any caller that constructs animations with raw sizes (e.g. `animations::create` in tests or gallery). ARC-001 later folds the startup/switch paths into one `rebuild_canvas`; leave clear `// MIN_TERM_*` references so the refactor preserves the policy.
- **Verify**: `cargo test every_animation_survives_all_sizes` passes; `cargo test` all green; manual: run `./target/release/termflix aurora` in a tmux split resized to 8x3 — no panic.

### [ARC-021] Remove dead code behind `#[allow(dead_code)]` (absorbs QA-010) — separate commit
- **Files**: `src/render/canvas.rs` (`Canvas::set` ~114), `src/generators/mod.rs` (`emit_at` ~146, `count` ~241, `clear` ~247), `src/record.rs` (`frames` ~76 — used; remove only the stale allow), `src/animations/tetris.rs` (~100, ~125, ~237), `src/animations/crystallize.rs` (~53), `src/animations/automata.rs` (~7, ~9), `src/animations/reaction_diffusion.rs` (~10, ~12), `src/animations/mod.rs` (~87 `supported_params` default)
- **Steps**:
  1. For each site run parsight `get_symbol_context` (repository_id `termflix`) to confirm zero non-test callers.
  2. Test-only helpers (`Canvas::set`) → `#[cfg(test)]`.
  3. Unused fns/fields → delete. For `automata.rs` `Ruleset.name`/`notation`: delete the fields and their initializers.
  4. `animations/mod.rs:87` `supported_params`: it is called by tests and is a trait default — keep it, remove only the allow if clippy is then clean; if clippy flags it, keep the allow with a one-line reason (ARC-013/ENH-005 gives it a runtime caller).
- **Method**: Rule R1: this lands as its own commit before ARC-001. Do not touch non-flagged code.
- **Verify**: `grep -rn "allow(dead_code)" src | wc -l` decreases to ≤ 1; `make checkall`.

### [ARC-012] Hoist `hsv_to_rgb` and near-duplicate helpers (absorbs QA-005, QA-014 color/neighbor parts)
- **Files**: new `src/color.rs`; `src/main.rs` (add `mod color;`); 21 animation files: find with `grep -ln "fn hsv_to_rgb" src/animations/`
- **Steps**:
  1. Diff variants: `for f in $(grep -ln "fn hsv_to_rgb" src/animations/); do echo "== $f"; sed -n "/fn hsv_to_rgb/,/^}/p" $f; done`. Expect 4 shapes (none / `rem_euclid` / `((h%1)+1)%1` / voronoi).
  2. Create `src/color.rs`:
     ```rust
     //! Shared color conversions for animations.

     /// HSV → RGB. `h` wraps (any real value), `s`/`v` in [0, 1].
     pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) { let h = h.rem_euclid(1.0); /* body of the canonical copy */ }
     ```
     Keep the exact return type of the existing copies (check whether they return `(u8,u8,u8)` or `(f64,f64,f64)`; if variants differ, provide both names and pick per call site).
  3. In each file delete the local fn and add `use crate::color::hsv_to_rgb;`.
  4. voronoi: if its variant differs in output (not only normalization), keep it local and document why.
  5. Add unit tests in `color.rs`: `hsv_to_rgb(0.0,1,1)==(255,0,0)`, `hsv_to_rgb(1.0,1,1)==hsv_to_rgb(0.0,1,1)`, `hsv_to_rgb(-0.25,1,1)==hsv_to_rgb(0.75,1,1)`.
  6. Optional in same change: `count_neighbors` toroidal helper shared by `life.rs` and `automata.rs` if signatures match exactly; otherwise skip (QA-014 remainder stays in 3c).
- **Method**: For inputs in [0,1) the wrapped version is identical to all variants, so output is unchanged. Only out-of-range hues change (the bug fix). Keep the gallery snapshot/encoder tests green.
- **Verify**: `grep -rn "fn hsv_to_rgb" src/animations | wc -l` ≤ 1; `cargo test color`; `make checkall`.

### [ARC-001] Decompose `run_loop`/`main`; single spawn helper (absorbs QA-003, QA-019, ARC-008)
- **Files**: `src/main.rs` (whole file), `src/gallery.rs` (~85-94), `src/animations/mod.rs` (macro)
- **Steps** (commit after each numbered step that leaves the tree green):
  1. Registry: extend `declare_animations!` so `ANIMATIONS` (or a new `fn preferred_render(name) -> RenderMode`) exposes each animation's preferred render without construction. Simplest: add a helper that constructs at 1x1-ish canvas once — no; better: add `$render:expr` as a 4th tuple element and fill it from the current `preferred_render()` impls (grep `fn preferred_render` across animations; default HalfBlock). Keep the trait method, implemented by returning the registry value, or remove it and update callers. Test that the registry value equals each impl's value before deleting impls.
  2. `fn spawn_animation(name: &str, canvas: &Canvas, scale: f64) -> Box<dyn Animation>` — `create(...).expect(...)` plus nothing else. Drop the redundant `on_resize` after construction (the constructor already receives the size). Replace all five sites.
  3. `struct Settings { anim_name, render_override, color_mode, color_quant, unlimited, frame_dur, scale, cycle, clean, screensaver, screensaver_keys, record_path, data_file, postproc, smoothing_tau, default_smoothing_tau, default_bloom, assist, dither, profile, single_threaded, full_frames }` + `fn resolve_settings(cli: &Cli, cfg: &Config) -> Settings`, built from the merge block at `main.rs:280-389`. `run_loop(settings: Settings, keybindings: &KeyBindings)`.
  4. `struct LoopState` for mutable runtime: `render_mode, color_mode, hide_status, postproc, smoothing_tau, dither, scale, anim_index, transition, prev_grid, cols, rows, needs_rebuild, resize_cooldown, adaptive_frame_dur, write_time_ema, frame_count, actual_fps, ...`.
  5. Extract, in order: `fn rebuild_canvas(state: &mut LoopState, ...) -> (Canvas, Box<dyn Animation>)` (the single place applying `MIN_TERM_*`, `color_quant`, `dither`), `fn handle_key(state, key, bindings) -> LoopAction` (enum `Continue | Quit | Rebuild | Switch(usize)`), `fn step_transition`, `fn status_line(state, cols) -> String`, `fn render_frame(...) -> Vec<u8>`, `fn write_frame(...)`.
  6. Remove `#[allow(clippy::too_many_arguments)]` from `run_loop`.
- **Method**: Pure refactor, no behavior change, except ARC-002 which falls out of `LoopState.dither`. Use parsight `get_impact` on `run_loop` and `animations::create` before starting. Test extracted pure pieces (`status_line`, `handle_key` → action) with unit tests. The QA-002 floor must be applied inside `rebuild_canvas` and at startup through it. Manual smoke after each step: `cargo run --release -- fire` then press `←/→ r c h b s d q`.
- **Verify**: `make checkall`; `cargo test every_animation_survives_all_sizes`; `grep -c "animations::create" src/main.rs` == 1; `run_loop` complexity via parsight `calculate_cyclomatic_complexity` target `run_loop` < 30.

### [ARC-002] Persist the dither toggle across rebuilds
- **Files**: `src/main.rs` (`d` handler ~785-787; rebuild sites ~645, ~831)
- **Steps**: If ARC-001 is done: `d` flips `state.dither`, and `rebuild_canvas` applies it. If done before ARC-001: add `let mut dither = dither;` at the top of `run_loop`, flip it in the `d` handler along with `canvas.dither`, and use it at every `canvas.dither = dither` site.
- **Verify**: Unit test on `handle_key`/`rebuild_canvas` if extracted; manual: press `d`, then `→`, status bar still shows dither on.

---

## Phase 3a — Security (remaining, parallel)

### [SEC-002] Checksummed installer (do together with ARC-006 — same `release.yml` change)
- **Files**: `install.sh`, `.github/workflows/release.yml` (`github-release` job ~226-280)
- **Steps**:
  1. release.yml `github-release`: after artifacts download, `cd artifacts && sha256sum termflix-* > SHA256SUMS` (match the actual artifact filenames used in the upload step), and upload `SHA256SUMS` with the binaries.
  2. install.sh: all `curl -sL` → `curl -fsSL`. Validate `VERSION` with `[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "bad version"; exit 1; }`. Download `SHA256SUMS`, then verify: `(cd "$TMP" && grep " $ASSET\$" SHA256SUMS | (command -v sha256sum >/dev/null && sha256sum -c - || shasum -a 256 -c -))` before `mv`. Fail closed.
  3. Remove the `xattr -d com.apple.quarantine` / `xattr -cr` block. A curl-downloaded file is not quarantined by macOS anyway (curl does not set the attribute), so removal does not break installs; add a comment-free note in README if a Gatekeeper prompt appears.
  4. Older releases lack `SHA256SUMS`: when the file 404s, print a clear warning and require `TERMFLIX_INSECURE=1` to continue.
- **Method**: Keep the script POSIX-bash compatible (it runs on macOS bash 3.2 — no `mapfile`, no `${var,,}`). `[[ =~ ]]` works in bash 3.2.
- **Verify**: `bash -n install.sh`; `shellcheck install.sh` if installed; `actionlint .github/workflows/release.yml` if installed; dry run: `INSTALL_DIR=$SCRATCH/bin bash install.sh` against the latest release (expect the insecure-warning path until a release with SHA256SUMS exists).

### [SEC-006] ndjson path guidance and bounded reads
- **Files**: `src/config.rs` (~153 template), `src/external.rs` (~120, ~137, ~162), `docs/EXTERNAL_ANIMATION.md` (examples using `/tmp/termflix.json`)
- **Steps**: Template/docs example → `~/.cache/termflix/control.json`. File reader: `File::open(path)?.take(64 * 1024).read_to_string(&mut s)`. Stdin reader: use `BufRead::read_line` in a loop with a 64 KiB cap (skip over-long lines).
- **Verify**: `cargo test external`.

### [SEC-009] `// SAFETY:` comments
- **Files**: `src/main.rs` (unsafe blocks at ~316, ~424, ~440 — may have moved into `restore_terminal` after ARC-015)
- **Steps**: One `// SAFETY:` line per block: static buffer, fd owned by this process, return value intentionally ignored during teardown.
- **Verify**: `make checkall`.

### [SEC-010] Bound gallery dimensions
- **Files**: `src/main.rs` (`Cli` gallery_cols/rows)
- **Steps**: `#[arg(long, value_parser = clap::value_parser!(u16).range(1..=1000))]` on both, typed `Option<u16>`, converted with `as usize` at use.
- **Verify**: `./target/release/termflix --gallery fire --gallery-cols 0; echo EXIT=$?` → 2 with a clap error.

---

## Phase 3b — Architecture (remaining, parallel)

### [ARC-005] `rust-version` and `include` (absorbs DOC-002)
- **Files**: `Cargo.toml`, `README.md` (~117), `CLAUDE.md` (~22)
- **Steps**: Add `rust-version = "1.88"` under `[package]`. Add `include = ["src/**/*", "Cargo.toml", "Cargo.lock", "README.md", "LICENSE", "CHANGELOG.md"]`. README/CLAUDE.md: replace "1.85+" with "see `rust-version` in Cargo.toml (currently 1.88)" or just the pointer.
- **Method**: 1.88 is the floor from let-chains (`src/external.rs:138`, `src/main.rs:263`). `cargo metadata` shows no dependency declaring > 1.85. If `rustup toolchain list` has 1.88 available, confirm with `cargo +1.88 check`; otherwise note it unverified.
- **Verify**: `cargo package --list --allow-dirty | grep -vE '^(src/|Cargo|README|LICENSE|CHANGELOG)'` prints nothing (aside from `.cargo_vcs_info.json`); `cargo +1.88 check` if the toolchain is installed.

### [ARC-006] Release workflow: gate, idempotent release, pins, permissions (absorbs SEC-005; combine with SEC-002)
- **Files**: `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `.github/workflows/gallery.yml`
- **Steps**:
  1. ci.yml: add `workflow_call:` to `on:` so release.yml can reuse it.
  2. release.yml: add job `ci: uses: ./.github/workflows/ci.yml`; add `ci` to `needs:` of `publish-crates` and `github-release`.
  3. Top-level `permissions: contents: read`; keep `permissions: contents: write` on `github-release` only; delete `id-token: write`.
  4. Replace `gh release delete ... && gh release create` with: `if gh release view "$VERSION" ...; then gh release upload "$VERSION" ... --clobber; else gh release create "$VERSION" ...; fi`.
  5. `dtolnay/rust-toolchain@master` → pin to a commit SHA with a `# master as of 2026-09-27` comment: resolve with `gh api repos/dtolnay/rust-toolchain/commits/master --jq .sha`. Same in ci.yml and gallery.yml. Pin `actions/checkout@v7`, `upload-artifact@v7`, `download-artifact@v8` only if the repo convention prefers SHAs (see `~/.claude/guides/git-ci.md`); otherwise leave the tags.
  6. Replace `Ilshidur/action-discord@master` with a `curl -fsS -H 'Content-Type: application/json' -d "$(jq -n --arg c "termflix $VERSION released" '{content:$c}')" "$DISCORD_WEBHOOK"` step, `env: DISCORD_WEBHOOK: ${{ secrets.DISCORD_WEBHOOK }}`.
  7. `cargo install cross --git ...` → `cargo install cross --locked --version <latest>` (check `cargo search cross --limit 1`; if the crates.io version lags, use `--git ... --tag vX.Y.Z --locked`).
- **Method**: Read `~/.claude/guides/git-ci.md` first (pinning rules). Every ref must resolve — verify each with `gh api`. Do not trigger the workflow.
- **Verify**: `actionlint` on all three files (install via `brew install actionlint` only if the user approves; otherwise `python -c 'import yaml,sys;yaml.safe_load(open(sys.argv[1]))'` for syntax); `grep -n "@master" .github/workflows/*.yml` prints nothing.

### [ARC-009] Single parser for RenderMode/ColorMode
- **Files**: `src/render/canvas.rs` (enums ~9-31), `src/config.rs` (`RenderModeConfig`/`ColorModeConfig` ~50-87), `src/main.rs` (`parse_render_mode`/`parse_color_mode` ~1204-1221, `--list` text ~275-276)
- **Steps**: `impl FromStr` for both enums accepting all spellings currently accepted anywhere (collect from clap `ValueEnum` names, config serde renames, and the `parse_*` matches — e.g. `half-block`, `halfblock`, `true-color`, `truecolor`). Derive `serde::Deserialize` via `#[serde(try_from = "String")]` + `TryFrom<String>` delegating to `FromStr`. Delete mirror enums and `parse_*`. Keep `clap::ValueEnum` or switch clap to `value_parser = str::parse::<RenderMode>` (removes the clap import from `render`). `--list` prints names from a `const ALL: &[...]` with `Display`.
- **Verify**: Unit tests for every spelling in all three paths; existing config tests pass; `make checkall`.

### [ARC-010] Keybinding actions with modifiers
- **Files**: `src/main.rs` (`KeyBindings`, `parse_key_binding`, `build_keybindings` ~1223-1311; hardcoded `b`/`s`/`d` ~771-787; status bar ~1046), `src/config.rs` (keybinding config), `src/record.rs` (`Player::play` quit)
- **Steps**: `enum Action { Next, Prev, Quit, Render, Color, Status, Bloom, Smooth, Dither }`. `struct KeyBindings(HashMap<Action, Vec<(KeyCode, KeyModifiers)>>)` with `fn action_for(&KeyEvent) -> Option<Action>` matching modifiers exactly (treat SHIFT on `Char` leniently: compare `c.to_ascii_lowercase()` when the binding has no SHIFT). One table `[(config_key, Action, defaults)]` drives defaults and config parsing. Config keys `bloom`, `smooth`, `dither` added (optional). Status-bar hint generated from bindings. `Player::play` takes `&KeyBindings` (quit action).
- **Method**: Keep `Ctrl+C` hard-wired as quit. Preserve replace-not-append semantics (document in DOC-005) unless the user wants append.
- **Verify**: Unit tests: `ctrl+q` binds with CONTROL and plain `q` no longer triggers it when rebound; defaults unchanged; `make checkall`.

### [ARC-011] Encapsulate `Canvas` fields
- **Files**: `src/render/canvas.rs` (~42-65), `src/gallery.rs` (~116-118), animations indexing `pixels`/`colors` directly (find with `grep -ln "canvas\.\(pixels\|colors\)\[" src/animations`)
- **Steps**: Make buffers `pub(crate)` first (no external crate consumers — it's a binary), then add `fn get(&self,x,y)->f64`, `fn pixels(&self)->&[f64]`, `fn rgb_frame(&self)->Vec<(u8,u8,u8)>`; move direct-index callers to accessors. `width`/`height` stay `pub` for now (147 in-degree — changing them is churn without payoff); document that.
- **Verify**: `make checkall`; encoder snapshot tests pass.

### [ARC-013] External state single-copy and error visibility (absorbs QA-015)
- **Files**: `src/external.rs` (~13-56, ~112-170), `src/config.rs` (`load_config` ~106)
- **Steps**: Remove the `params` mirror in `CurrentState`; compute `params()` from fields. `load_config`: on `Err(e)` where `e.kind() != NotFound`, `eprintln!("termflix: could not read config {path}: {e}")` (this runs before raw mode — verify call order in `main`). For ndjson parse failures: store the last error string in `CurrentState` (`last_error: Option<String>`) and show it in the status bar for 3 s instead of printing (raw mode).
- **Verify**: `cargo test external config`; manual: pipe `echo '{bad' | termflix fire` shows the status-bar error.

### [ARC-019] Remove no-op `#[allow(unused_variables)]`; document `scale` (absorbs QA-009)
- **Files**: ~25 animation files (`grep -ln "allow(unused_variables)" src/animations`), `README.md`
- **Steps**: Delete each attribute; run `cargo clippy` to confirm no new warnings (params are `_`-prefixed). Build the list of animations that read `scale` (`grep -L "_scale" src/animations/*.rs` minus mod.rs) and add a sentence to the README `--scale` description.
- **Verify**: `grep -rn "allow(unused_variables)" src | wc -l` == 0; `make checkall`.

### [ARC-015] Shared `restore_terminal`
- **Files**: `src/main.rs` (panic hook ~309-326, exit ~417-456)
- **Steps**: `fn restore_terminal(screensaver: bool)` with the cfg-split body; call from the panic hook (capture `screensaver` by move) and at exit. Keep byte order `\x1b[?2026l` first (see the existing comment — it is a real constraint).
- **Verify**: `make checkall`; manual quit and a forced panic (temporary `panic!` in a scratch build, not committed) both restore the terminal.

### [ARC-016] Save the recording after terminal restore
- **Files**: `src/main.rs` (~725-733)
- **Steps**: `run_loop` returns the `Option<Recorder>` (or a `LoopOutcome`); `main` saves after `restore_terminal`.
- **Verify**: `termflix fire --record $SCRATCH/r.asciianim` then `q` prints "Saved …" on the normal screen.

### [ARC-017] Collapse the legacy render path
- **Files**: `src/render/canvas.rs` (`render` ~141-147, `render_cells` ~187), `src/render/braille.rs` (~26), `src/render/halfblock.rs` (~10), `src/animations/matrix.rs` test (~312)
- **Steps**: Replace `render_cells` callers with `build_grid`; mark `render()` functions `#[cfg(test)]` or port the tests to `build_grid`.
- **Verify**: `make checkall`.

### [ARC-022] Repo hygiene (absorbs QA-022, DOC-020 root files)
- **Files**: `.gitignore`, `src/main.rs` (`pub mod generators` ~5), `ideas.md~`, `.gitignore~`
- **Steps**: Remove `.githooks/` and `ideas.md` from `.gitignore` (they are tracked). Add `*~`. Delete the two untracked `~` files (they are untracked editor backups; confirm with `git status --ignored`). `pub mod generators` → `mod generators`. Leave `ideas.md`/`reddit_release.md` for DOC-020.
- **Verify**: `git status` clean except intended changes; `make checkall`.

### [ARC-023] Recording header carries dimensions
- **Files**: `src/record.rs`, `src/main.rs` (`detect_recording_size`)
- **Steps**: Recorder writes `SIZE <cols> <rows>` after `FRAMES`. Loader accepts it optionally (v1 files without it still load). `--export-gif` uses the header size when present (clamped by SEC-001's limits), else `detect_recording_size`.
- **Verify**: `cargo test record`; round-trip test.

---

## Phase 3c — Code Quality (remaining, parallel; after ARC-012 for animation files)

### [QA-006] Allocation-free color escapes
- **Files**: `src/render/canvas.rs` (`color_to_fg`/`color_to_bg` ~435), `src/render/encoder.rs` (~49, ~60, ~66)
- **Steps**: Add `pub fn write_fg(out: &mut String, c: (u8,u8,u8), mode: ColorMode)` and `write_bg` using `use std::fmt::Write; let _ = write!(out, "38;2;{};{};{}", r, g, b);` (mirror exact current output per mode). Switch encoder call sites. Delete the String-returning versions if no callers remain.
- **Method**: Output bytes must be identical; the encoder snapshot tests guard this. Run `cargo test --release -- --ignored bench_dirty` before and after, record both numbers in the commit message.
- **Verify**: `cargo test encoder`; bench shows no regression.

### [QA-007] Shared `GifWriter` (after SEC-001)
- **Files**: `src/gif.rs` (`export_gif` ~494-661, `export_gif_pixels` ~670-797)
- **Steps**: `struct GifWriter<W: Write> { w: W, width: u16, height: u16 }` with `new(w, width, height, palette) -> io::Result<Self>` (header, LSD, global palette, NETSCAPE loop), `write_frame(&mut self, indices: &[u8], delay_cs: u16)` (GCE, image descriptor, LZW, sub-blocks), `finish(self)` (trailer). Both exporters convert to indices then call it. Remove `let _ = frame_count;`.
- **Verify**: `cargo test gif` (existing byte-level tests must pass unchanged).

### [QA-008] Split complex game `update`/`draw`
- **Files**: `rainforest.rs` (~233), `hackerman.rs` (~170), `maze.rs` (`draw` ~265), `flappy_bird.rs` (~128), `cells.rs` (~106), `tetris.rs` (`render` ~614, `piece_cells` ~44), `garden.rs` (~222), `invaders.rs` (~90), `pong.rs` (~76)
- **Steps**: Per file: extract `fn simulate(&mut self, dt)` and `fn draw(&self, canvas)`, then per-layer draw helpers. tetris: `const SHAPES: [[[(i8,i8);4];4];7]` indexed `[piece][rotation]`. One commit per file.
- **Method**: Pure refactor; QA-004's size sweep is the regression guard, plus a gallery capture diff for the file (`--gallery <name>`; compare PNG sizes/visually — RNG is unseeded so pixel diffs are not exact).
- **Verify**: parsight `calculate_cyclomatic_complexity` on each method < 25; `make checkall`.

### [QA-011] Checked index/size arithmetic
- **Files**: `src/animations/aurora.rs` (~34-35; may be done by QA-002), `src/main.rs` (~532 p95 index)
- **Steps**: `let p95_idx = ((n as f64) * 0.95).ceil() as usize; let p95_idx = p95_idx.saturating_sub(1).min(n - 1);`.
- **Verify**: `make checkall`.

### [QA-012] NaN-safe profiler sort
- **Files**: `src/main.rs` (~530)
- **Steps**: `sorted.sort_by(f64::total_cmp);`
- **Verify**: `make checkall`.

### [QA-013] Parameter structs for too-many-arguments
- **Files**: `strange_attractor.rs` (~109), `cells.rs` (~382), `lightning.rs` (~58), `matrix.rs` (~24 `create_drops`), `sort.rs` (~6), `sandstorm.rs` (~140); `grep -rn "too_many_arguments" src` for the full list
- **Steps**: e.g. matrix `struct DropSpec { count: usize, len: (usize, usize), speed: (f64, f64) }`. Remove the allow.
- **Verify**: `grep -rn "too_many_arguments" src | wc -l` == 0 (after ARC-001 removes the main.rs one); `make checkall`.

### [QA-014] Shared spawn scaffolding (remainder)
- **Files**: `nbody.rs` (~46, ~80), `campfire.rs`/`particles.rs`/`smoke.rs` (`new`), `rain.rs`/`waterfall.rs` (`new`)
- **Steps**: `spawn_initial_bodies` calls `spawn_body(spread)`. For the particle trio, only extract if an `EmitterConfig` preset constructor in `src/generators/mod.rs` makes each `new` shorter; otherwise leave (genuine variation).
- **Verify**: `make checkall`.

### [QA-016] Bloom scratch buffer
- **Files**: `src/render/canvas.rs` (~268, ~202)
- **Steps**: `bloom_scratch: Vec<f64>` field (private), resized with `resize(len, 0.0)` per frame.
- **Verify**: `cargo test canvas`.

### [QA-018] Friendly `--init-config` error
- **Files**: `src/main.rs` (~181)
- **Steps**: `let Some(path) = config::config_path() else { return Err(io::Error::new(io::ErrorKind::NotFound, "could not determine config directory")); };`
- **Verify**: `make checkall`.

### [QA-020] `name()` returns `&'static str`
- **Files**: `src/animations/mod.rs` (trait), all 60 animation files
- **Steps**: Change trait signature; update impls (`sed -i '' 's/fn name(&self) -> &str/fn name(\&self) -> \&'"'"'static str/'` then fix any impl returning a non-static). Do after ARC-012/QA-008 to avoid churn conflicts.
- **Verify**: `make checkall`.

### [QA-021] Single-char key parsing
- **Files**: `src/main.rs` (`parse_key_code` ~1249)
- **Steps**: `s if s.chars().count() == 1 => s.chars().next().map(KeyCode::Char),`. Fold into ARC-010 if that lands first.
- **Verify**: Unit test `parse_key_code("é") == Some(KeyCode::Char('é'))`.

---

## Phase 3d — Documentation (parallel; code-dependent items noted)

### [DOC-001] README `INSTALL_DIR` example
- **Files**: `README.md` (~106)
- **Steps**: `curl -fsSL https://raw.githubusercontent.com/paulrobello/termflix/main/install.sh | INSTALL_DIR=~/.local/bin bash`. Also switch the main example to `-fsSL`.
- **Verify**: `grep -n "INSTALL_DIR" README.md` shows the variable after the pipe.

### [DOC-003] Per-OS config path
- **Files**: `src/main.rs` (`init_config` help ~85), `src/config.rs` (~96 doc comment), `CLAUDE.md` (~40), `docs/ARCHITECTURE.md` (~117, ~501), `docs/EXTERNAL_ANIMATION.md` (~372), `reddit_release.md` (~28 — or leave, see DOC-020)
- **Steps**: Help: "Generate default config file (see --show-config for its path)". Docs: a 3-row table — Linux `~/.config/termflix/config.toml`, macOS `~/Library/Application Support/termflix/config.toml`, Windows `%APPDATA%\termflix\config.toml` — plus "run `termflix --show-config`". Copy README.md:272-275 wording for consistency.
- **Verify**: `grep -rn "\.config/termflix" --include=*.md --include=*.rs . | grep -v README` shows only rows that are explicitly Linux.

### [DOC-004] Bloom/smoothing defaults
- **Files**: `docs/ARCHITECTURE.md` (~319, ~533)
- **Steps**: Default `0.0 (off)`; note: "`b` toggles bloom between off and the configured value (0.4 if unset); `s` toggles smoothing between off and the configured value (0.1 if unset)".
- **Verify**: Matches `src/main.rs:346-380`.

### [DOC-005] Keybinding reference (after ARC-010)
- **Files**: `README.md` (~329-336), `CHANGELOG.md` (~74), `src/config.rs` template (~155)
- **Steps**: If ARC-010 has landed: document actions, key names (`Left Right Up Down Esc Enter Space Tab` + single chars), modifier syntax, replace-not-append, `Ctrl+C` always quits. If not: document current limits (modifiers ignored, `b/s/d` fixed) and add a CHANGELOG `[Unreleased]` correction note for the 0.5.0 claim.
- **Verify**: Each documented key name appears in `parse_key_code`.

### [DOC-006] README CLI reference, docs links, stdin, TOC
- **Files**: `README.md`
- **Steps**: Add "CLI Reference" table generated from `./target/release/termflix --help` (flag, short, description). Add "Documentation" section linking `docs/EXTERNAL_ANIMATION.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`, `CONTRIBUTING.md` (after DOC-014). Add `some-generator | termflix plasma` stdin example. Add a TOC after the intro.
- **Verify**: Every `--flag` in `termflix --help` appears in README: `./target/release/termflix --help | grep -oE -- '--[a-z-]+' | sort -u | while read f; do grep -q -- "$f" README.md || echo "missing $f"; done` prints nothing.

### [DOC-007] Per-animation external ranges
- **Files**: `docs/EXTERNAL_ANIMATION.md` (~126-132, ~345-362)
- **Steps**: Table: animation | field | internal target | accepted range | direction (e.g. snake | speed | move interval | 0.02–0.2 | higher = slower). Sources: `snake.rs:154-155`, `sort.rs:220`, `boids.rs:118`, `particles.rs:75`, wave's `set_params`. Add a warning that these override the global meaning. If ENH-005 lands later, update.
- **Verify**: Each row cites a `set_params` clamp that matches the code.

### [ARC-014] Thread model and animation count (text)
- **Files**: `CLAUDE.md` (~22, ~25, ~36), `docs/ARCHITECTURE.md` (~38-40, ~983, ~1000)
- **Steps**: "Main thread; a writer thread (`render_sink::ThreadedRenderer`, on by default on Unix, disabled by `--single-threaded`); an optional external-reader thread (stdin or notify watcher)." Count: "60 animations" — better: "see `termflix --list`" to avoid future drift.
- **Verify**: `grep -rn "55\|one optional background thread\|pure synchronous" CLAUDE.md docs/ARCHITECTURE.md` shows no stale claims.

### [DOC-008] CLAUDE.md pipeline, CI, module list (after ARC-001 ideally)
- **Files**: `CLAUDE.md`
- **Steps**: Pipeline: `update → smoothing → effects → color assist → post_process → build_grid → encode_diff → writer thread`. CI: gallery.yml runs on push to main (and ci.yml after ARC-003). Module list: add `gallery.rs`, `png.rs`, `render_sink.rs`, `render/encoder.rs`, `render/cell.rs`, `render/color_assist.rs`, `color.rs` (after ARC-012). Tag the fence `text`.
- **Verify**: Every `src/*.rs` and `src/render/*.rs` file is listed: `ls src/*.rs src/render/*.rs | xargs -n1 basename | while read f; do grep -q "$f" CLAUDE.md || echo "missing $f"; done`.

### [DOC-009] ARCHITECTURE.md v0.8.0 details
- **Files**: `docs/ARCHITECTURE.md` (~48-136, ~213-225, ~280-282, ~461)
- **Steps**: Add `png.rs` to module diagram and tree; add `prev_pixels` to the `Canvas` snippet; document wide-glyph routing (`grid_has_wide` in `src/render/encoder.rs`, frames with wide glyphs skip `encode_diff`) in the ASCII renderer and dirty-cell sections.
- **Verify**: `grep -n "png.rs\|prev_pixels\|grid_has_wide" docs/ARCHITECTURE.md` hits all three.

### [DOC-010] matrix description
- **Files**: `src/animations/mod.rs` (~116), `docs/ARCHITECTURE.md` (~916)
- **Steps**: Use README.md:35's wording (kana/Latin glyph rain) in both.
- **Verify**: `./target/release/termflix --list matrix` matches README.

### [DOC-011] Watcher error behavior (after ARC-013)
- **Files**: `docs/EXTERNAL_ANIMATION.md` (~509, ~547)
- **Steps**: Describe how watcher-setup failures and parse errors surface (status bar after ARC-013, or stderr-over-animation today). Remove or correct the "file deleted → thread exits" row: the loop keeps receiving events and re-reads fail silently.
- **Verify**: Statements match `src/external.rs` at the time of editing.

### [DOC-012] Hotkey table
- **Files**: `docs/EXTERNAL_ANIMATION.md` (~493-501)
- **Steps**: Replace the table with a link to the README hotkeys section (single source).
- **Verify**: Link resolves (`find_broken_doc_links`).

### [DOC-013] README example config
- **Files**: `README.md` (~279-301)
- **Steps**: Comment out `render = "half-block"` (and `animation`), add "leave `render` unset to keep each animation's preferred mode".
- **Verify**: Matches `src/config.rs` template (~125-130).

### [DOC-014] CONTRIBUTING.md
- **Files**: new `CONTRIBUTING.md`, `README.md` (~355-367)
- **Steps**: Contents: setup (`make hooks`), `make checkall`, adding an animation (create file; `pub mod`; `declare_animations!` entry; `fn new(width, height, scale) -> Self`; add to README table), commit style, PR flow. README section shrinks to a link.
- **Verify**: Steps match CLAUDE.md "Adding a New Animation".

### [DOC-015] CHANGELOG hygiene
- **Files**: `CHANGELOG.md`
- **Steps**: Add `## [Unreleased]` with `dirs` 6→7 and CI action bumps (commit `1c45147`) plus any fixes landing from this audit. Add a Keep a Changelog/SemVer line. Fix 0.5.0 render-mode text for maze/rainforest (HalfBlock). Fix 0.4.2 `serde_json` claim to `"1"`. Add compare links; note `v0.5.0`/`v0.7.0` tags are missing (do not create tags — outward-facing).
- **Verify**: `git tag -l` vs headings reviewed; links well-formed.

### [DOC-016] Rustdoc on public items
- **Files**: `src/external.rs`, `src/gallery.rs`, `src/render/canvas.rs`, `src/render/cell.rs`, `src/render/braille.rs`, `src/render/halfblock.rs`, `src/render/encoder.rs`, `src/render_sink.rs`, `src/gif.rs`, `src/config.rs`
- **Steps**: `//!` module headers on external, gallery, render_sink, render/encoder, render/canvas. `///` on `ExternalParams` (one-shot vs persistent fields), `CurrentState::merge`/`take_*`, `spawn_reader` (thread lifetime, errors), `PostProcessConfig` fields with ranges, `Canvas::new` (sub-pixel dims per mode), `CellGrid`, `GalleryConfig`, `run_gallery`. Run after ARC-011/ARC-013 if those rename items.
- **Verify**: parsight `list_symbols` with `documented: false, visibility: "public"` count drops substantially; `cargo doc --no-deps` has no warnings.

### [DOC-017] Mermaid `classDef`
- **Files**: `docs/ARCHITECTURE.md`, `docs/EXTERNAL_ANIMATION.md`
- **Steps**: Replace per-node `style` lines with `classDef` + `class` using the dark palette (`#1E1E1E` bg, `#E6E6E6` text, success/warning/error/info colors).
- **Verify**: `grep -c "^\s*style " docs/*.md` == 0.

### [DOC-018] Fence tags and callouts
- **Files**: `docs/ARCHITECTURE.md` (~114, ~268, ~425, ~447, ~629), `docs/EXTERNAL_ANIMATION.md`, `CLAUDE.md` (~26)
- **Steps**: Tag bare fences `text`; callouts `**Note:**` / `**Tip:**` / `**Warning:**` without emoji.
- **Verify**: `grep -n '^```$' docs/*.md CLAUDE.md` only matches closing fences.

### [DOC-019] `src/` prefixes
- **Files**: `docs/ARCHITECTURE.md` (~142, ~176, ~264, ~276, ~323, ~461)
- **Steps**: Prefix source paths with `src/`.
- **Verify**: parsight `find_broken_doc_links` with `min_confidence: "medium"` no longer lists these.

### [DOC-020] Planning doc status and stale root docs
- **Files**: `docs/plans/*.md`, `docs/superpowers/plans/*.md`, `docs/superpowers/specs/*.md`, `ideas.md`, `reddit_release.md`
- **Steps**: Add `> Status: Implemented (vX.Y.Z)` to executed plans/specs (check CHANGELOG for the version). `ideas.md`: mark the "Auto-Sync Animation Count & Table" docs item done (README already matches the registry) or delete it. `reddit_release.md`: add a dated header "Snapshot for v0.5.1 — not maintained" or move to `docs/archive/`.
- **Verify**: `grep -L "Status:" docs/plans/*.md docs/superpowers/*/*.md` prints nothing.

### [DOC-021] Troubleshooting section
- **Files**: `README.md`
- **Steps**: Entries: config has no effect (wrong OS path, unknown keys silently ignored → `--show-config`); garbled glyphs (font lacks braille/kana → `--render half-block`); Windows has no writer thread; invalid `--palette`/`--colorblind` names ignored; tmux tearing (link existing section).
- **Verify**: Section present, each entry actionable.

### [DOC-022] AGENTS.md pointer
- **Files**: `AGENTS.md`
- **Steps**: Keep `@CLAUDE.md`; add a second line `See CLAUDE.md for project instructions.`
- **Verify**: File has both lines.

### [DOC-023] Badge and dated FPS figures
- **Files**: `README.md` (~3-6, ~255-258)
- **Steps**: Add a Gallery workflow badge (`https://github.com/paulrobello/termflix/actions/workflows/gallery.yml/badge.svg`); label FPS figures "(measured on v0.x)" — find the version from `git log -S` on that section.
- **Verify**: Badge URL returns 200 (`curl -fsI`).
