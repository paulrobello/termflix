//! Golden-frame regression tests: every animation rendered deterministically
//! at a fixed size, hashed, and compared against checked-in values.
//!
//! An unintended visual change (a refactor, a color-helper hoist, a pipeline
//! change) fails the test and names the animation. For an INTENTIONAL visual
//! change, regenerate with:
//!
//! ```text
//! TERMFLIX_UPDATE_GOLDEN=1 cargo test golden
//! ```
//!
//! and review the diff of `golden_hashes.txt` like any other code change.
//!
//! Determinism rests on ENH-001's thread-local seeded RNG (`crate::rng`) and
//! on quantizing floats before hashing: raw `f64` bits could differ across
//! platforms via libm (`sin`, `exp`), so pixel brightness is rounded to a
//! 1/1024 grid first. The cell-grid hash needs no quantization — chars and
//! 8-bit color triples are already discrete.

use super::{ANIMATION_NAMES, create, preferred_render};
use crate::color::color_to_rgb;
use crate::render::pipeline::{FrameEffects, produce_frame};
use crate::render::{Canvas, ColorAssist, ColorMode, PostProcessConfig};

const GOLDEN_SEED: u64 = 1234;
const COLS: usize = 40;
const ROWS: usize = 12;
const FRAMES: u32 = 48;
const DT: f64 = 1.0 / 24.0;

const GOLDEN_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/animations/golden_hashes.txt"
);

/// FNV-1a 64 over `data`, chained from `h`.
fn fnv1a64(mut h: u64, data: &[u8]) -> u64 {
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Render one animation through the shared pipeline under fixed conditions
/// and return its golden line: `name pixel_hash grid_hash` (hex).
fn golden_line(name: &str) -> String {
    crate::rng::set_seed(GOLDEN_SEED);
    let mode = preferred_render(name);
    let mut canvas = Canvas::new(COLS, ROWS, mode, ColorMode::TrueColor);
    let mut anim = create(name, canvas.width, canvas.height, 1.0)
        .unwrap_or_else(|| panic!("unknown animation {name}"));
    anim.on_resize(canvas.width, canvas.height);
    let postproc = PostProcessConfig::default();
    let fx = FrameEffects::neutral(&ColorAssist::None, &postproc);
    for f in 0..FRAMES {
        produce_frame(anim.as_mut(), &mut canvas, DT, f as f64 * DT, &fx);
    }

    // Pixel hash: quantized brightness + color triple per pixel.
    let mut ph: u64 = 0xcbf29ce4_84222225;
    for (&p, &(r, g, b)) in canvas.pixels.iter().zip(canvas.colors.iter()) {
        let q = (p.clamp(0.0, 4.0) * 1024.0).round() as u32;
        ph = fnv1a64(ph, &q.to_le_bytes());
        ph = fnv1a64(ph, &[r, g, b]);
    }

    // Grid hash: the terminal cells the frame would actually emit.
    let grid = canvas.build_grid();
    let mut gh: u64 = 0xcbf29ce4_84222225;
    for cell in &grid.cells {
        let mut char_buf = [0u8; 4];
        gh = fnv1a64(gh, cell.ch.encode_utf8(&mut char_buf).as_bytes());
        gh = fnv1a64(gh, &[cell.fg.is_some() as u8, cell.bg.is_some() as u8]);
        if let Some(c) = cell.fg {
            let (r, g, b) = color_to_rgb(c);
            gh = fnv1a64(gh, &[r, g, b]);
        }
        if let Some(c) = cell.bg {
            let (r, g, b) = color_to_rgb(c);
            gh = fnv1a64(gh, &[r, g, b]);
        }
    }

    format!("{name} {ph:016x} {gh:016x}")
}

#[test]
fn golden_frames_match() {
    let lines: Vec<String> = ANIMATION_NAMES.iter().map(|n| golden_line(n)).collect();

    if std::env::var_os("TERMFLIX_UPDATE_GOLDEN").is_some() {
        let mut out = lines.join("\n");
        out.push('\n');
        std::fs::write(GOLDEN_PATH, out).unwrap_or_else(|e| panic!("write {GOLDEN_PATH}: {e}"));
        eprintln!("rewrote {} golden lines into {GOLDEN_PATH}", lines.len());
        return;
    }

    let golden: std::collections::HashMap<&str, &str> = include_str!("golden_hashes.txt")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
            [name, _ph, _gh] => (name, l),
            _ => panic!("malformed golden line: {l}"),
        })
        .collect();

    assert_eq!(
        golden.len(),
        ANIMATION_NAMES.len(),
        "golden file line count"
    );
    for want in &lines {
        let name = want.split_whitespace().next().expect("name");
        let expected = golden.get(name).unwrap_or_else(|| {
            panic!(
                "golden entry missing for {name}: run TERMFLIX_UPDATE_GOLDEN=1 cargo test golden"
            )
        });
        assert_eq!(
            want, expected,
            "golden frame mismatch for {name}: if this change is intentional, \
             regenerate with TERMFLIX_UPDATE_GOLDEN=1 cargo test golden and \
             review the golden_hashes.txt diff"
        );
    }
}
