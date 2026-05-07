# termflix v0.5.1 — terminal animation player, now with a live gallery

**Repo:** https://github.com/paulrobello/termflix
**Live gallery:** https://paulrobello.github.io/termflix/
**crates.io:** https://crates.io/crates/termflix

termflix is a terminal animation player I write for fun: 54 procedurally generated animations (fire, matrix rain, plasma, ocean waves, automata, n-body, voronoi, self-playing tetris/snake/flappy-bird, and a bunch more), three render modes (braille / half-block / ASCII), 24-bit color, low CPU, plays nice in tmux. Pure synchronous Rust, no async, no GPU, no web.

This is a small follow-up to **0.5.0** that ships the thing I always wanted but kept putting off — **a browsable gallery** so you can actually see what every animation looks like before installing — plus three real bugs in the export pipeline.

## What's new

**`--gallery` capture mode**

```
termflix --gallery                      # capture all 54 animations
termflix --gallery fire,plasma,matrix   # subset
make gallery                            # repo convenience target
```

Captures every animation as a 640×400 PNG still and a 640×400 animated GIF at native canvas resolution, then writes a dark-themed `index.html` lightbox gallery. Fully offscreen — no real terminal needed — so it runs cleanly on a stock GitHub Actions Linux runner.

**Auto-deployed to GitHub Pages**

A new `Gallery` workflow runs `make gallery` on push to `main` (when source / build files change) and on manual dispatch, then publishes to Pages. The live site at https://paulrobello.github.io/termflix/ now stays in sync with the code automatically — every push refreshes the stills and GIFs.

## Bugs squashed (this is most of the work)

I'd been carrying a hand-written PNG encoder and a hand-written GIF89a encoder (no external image crates — that's part of the fun), and the gallery feature found three sharp edges in them:

- **PNG chunk CRC32 was wrong** for IHDR / IDAT — the code finalized the CRC over the chunk type, then continued accumulating data without un-finalizing or re-finalizing. `file(1)` accepted the result (loose parser), but every strict decoder rejected the PNGs as corrupted. Fixed by computing the CRC over `chunk_type ++ data` in a single pass with one finalize. Added a roundtrip test that walks every chunk in a generated PNG and re-validates its CRC.

- **LZW width-bump was off by one** — the encoder bumped code width when post-add `next_code > max_code` (= `> (1<<width) - 1`), one step too early. Symptom: the top ~25 rows of every GIF decoded fine and the rest went black, because the decoder was reading at width N while the encoder had already moved to width N+1. Standard giflib/Pillow rule is post-add `next_code > 1<<width`; the decoder uses `>= 1<<width` because its add lags the encoder's by one read — both produce the same logical bump point. Added LZW roundtrip tests (pseudo-random, long compressible runs, dictionary-fill / clear-and-reset path) since the previous test only checked output was non-empty.

- **GIF colors were lost** in the gallery path because the encoder went through a tiny `VirtualTerminal` that re-decoded ANSI to extract pixel data, but it didn't parse `48;…` background SGR (so half-block bottom pixels were dropped), and its SGR parser misread BG-RGB component zeros as the SGR-`0` reset code, clobbering the foreground to black. Fixed by adding a `gif::export_gif_pixels` that takes per-frame RGB pixel data straight from the canvas, computes palette indices at native resolution for cheap dedup, and upscales by nearest-neighbor when emitting LZW. Result: 640×400 with full color fidelity; before the fix gallery GIFs were 80×25 and grey.

## Changelog

[CHANGELOG.md → 0.5.1](https://github.com/paulrobello/termflix/blob/main/CHANGELOG.md)

## Try it

```
cargo install termflix
termflix              # default: fire animation
termflix --list       # see all 54
termflix matrix
termflix --auto-cycle 10
```

Or just open the gallery: https://paulrobello.github.io/termflix/

Feedback, bug reports, and animation ideas welcome.
