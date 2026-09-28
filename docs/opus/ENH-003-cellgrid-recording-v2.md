# ENH-003 — Recording format v2: cell-grid frames with explicit size

## Goal

`.asciianim` v2 stores terminal dimensions in the header and each frame as a cell grid (char + fg + bg), not raw ANSI text. Export to GIF reads cells directly without the hand-written VT emulator, playback re-encodes through the normal encoder (and so emits only escape sequences termflix itself produces, which structurally closes SEC-003), and recording no longer forces full-frame output. v1 files keep loading.

This overlaps the existing `ideas.md` item "[record] Portable Recording Format". This plan is the concrete, cell-level version of it. Storing pre-render pixel canvases, which would allow re-rendering in another mode, remains a possible follow-up, but it is a much larger file format and is out of scope.

## Current state

- `src/record.rs`: `Recorder` stores `Frame { timestamp_ms, content: String }` with ANSI text. `save` writes the `ASCIIANIM v1` header, then `FRAMES n`, then `---` / `T ms` / base64 blocks. `Player::load`/`play` replays the text verbatim.
- `src/main.rs:~998`: when `recorder.is_some()`, dirty-cell diffing is disabled so every frame is self-contained.
- `src/gif.rs:32-200`: `VirtualTerminal` parses ANSI to rebuild cells for `export_gif`.
- `src/main.rs:1171-1201` `detect_recording_size` guesses the size from CUP sequences in frame 1 (ARC-023, SEC-001).
- `src/render/cell.rs`: `CellGrid { cols, rows, cells }`, where a cell holds a char, fg and bg (read the file for exact types).
- Audit context: ARC-018, ARC-023, SEC-001, SEC-003.

## Design

- Header: `ASCIIANIM v2`, then `SIZE <cols> <rows>` and `FRAMES <n>`.
- Frame block: `---`, `T <ms>`, then one base64 line holding a compact binary encoding of the `CellGrid`. For each cell: UTF-8 char length (u8) + char bytes + fg (3 bytes) + bg (3 bytes) + flags (1 byte: has_fg, has_bg). Use run-length encoding of identical consecutive cells (a u16 run count) to keep files small.
- Place the encode/decode functions in `src/record.rs` next to the v1 code, as `fn encode_grid(&CellGrid) -> Vec<u8>` and `fn decode_grid(&[u8], cols, rows) -> io::Result<CellGrid>`. Decode validates lengths against `cols*rows` and returns `InvalidData` on any mismatch.
- `Recorder::push_grid(t, &CellGrid)` replaces the ANSI push. `run_loop` passes the `grid` it already builds, and the forced-full-frame condition for recording is removed.
- `Player`: `enum Frames { V1(Vec<TextFrame>), V2 { cols, rows, frames: Vec<GridFrame> } }`. v2 playback encodes each grid through `render::encoder` (full frame) and writes it, so output bytes are termflix-generated. v1 playback keeps the SEC-003-sanitized path.
- GIF export: v2 converts cells to pixels directly (the logic already exists inside `VirtualTerminal` after parsing, so extract it). v1 keeps `VirtualTerminal`.
- Limits (from SEC-001/SEC-008): `SIZE` ≤ 1000x500, `FRAMES` ≤ 100000.

## Steps

1. Read `src/record.rs`, `src/render/cell.rs`, the `VirtualTerminal` cell-to-pixel code in `src/gif.rs`, and the `recorder` handling in `run_loop`. Confirm whether SEC-001/003/004/008 have landed (they change `load`/`play`) and build on the current code.
2. Implement `encode_grid`/`decode_grid` with round-trip unit tests: an empty grid, a 1x1 grid, an 80x24 grid of random cells including wide chars and braille, and RLE boundaries (run of 1, run of 65535+). Check any wide-char handling in `CellGrid` (`grid_has_wide`) so continuation cells round-trip.
3. Writer: `Recorder` records grids and `save` writes v2. Keep the v1 writer only if something still needs it (nothing should).
4. Loader: dispatch on the header and parse `SIZE`. Reject v2 without `SIZE`.
5. Playback v2 via the encoder. The quit handling is shared with v1 (from SEC-004).
6. GIF export v2: extract a `cells_to_pixels(&CellGrid) -> Vec<(u8,u8,u8)>` from `gif.rs` and have both the v1 VT path and the v2 path use it. `detect_recording_size` is used only for v1.
7. Remove `&& recorder.is_none()` from the dirty-diff condition in `run_loop`.
8. Docs: README recording section (v2 format, v1 still readable), ARCHITECTURE.md recording section, CHANGELOG `[Unreleased]`.

## Files to touch

`src/record.rs`, `src/gif.rs`, `src/main.rs`, `src/render/cell.rs` (only if it needs a constructor or accessor), `README.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`.

Sequencing: after the Phase 1 security batch (SEC-001/003/004/007/008) and ARC-023 (that header field is superseded by `SIZE`, so ARC-023 can be closed by this plan if it has not landed).

## Verify

- `cargo test record` passes, including the `encode_grid`/`decode_grid` round-trip tests
- Round trip: `./target/release/termflix fire --record /tmp/r2.asciianim` (quit after about 3 s), then `head -2 /tmp/r2.asciianim` prints `ASCIIANIM v2` and `SIZE <cols> <rows>`
- `./target/release/termflix --play /tmp/r2.asciianim --export-gif /tmp/r2.gif; echo EXIT=$?` prints `EXIT=0` and the GIF opens
- A v1 fixture file (commit a small one under `tests/fixtures/` or build it inline in a test) still plays and exports
- A crafted v2 file with `SIZE 70000 70000` is rejected with an error, not an OOM
- `make checkall` exits 0

## Rollback

v1 loading is preserved, so reverting the writer restores v1 output. Files recorded as v2 would not play on a reverted build, so mention the format bump in the CHANGELOG.
