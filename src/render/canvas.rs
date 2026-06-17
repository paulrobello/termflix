use super::cell::{Cell, CellGrid};
use crossterm::style::Color;

/// How to render sub-cell pixels to terminal characters
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RenderMode {
    /// Unicode braille characters (2x4 per cell = highest resolution)
    Braille,
    /// Half-block characters ▀▄█ (1x2 per cell)
    HalfBlock,
    /// Plain ASCII characters with density mapping
    Ascii,
}

/// Color output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorMode {
    /// No color — monochrome
    Mono,
    /// ANSI 16 colors
    Ansi16,
    /// 256-color palette
    Ansi256,
    /// 24-bit true color (RGB)
    TrueColor,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PostProcessConfig {
    pub bloom: f64,
    pub bloom_threshold: f64,
    pub vignette: f64,
    pub scanlines: bool,
}

/// A pixel-level canvas that gets rendered to terminal characters.
/// Coordinates are in "sub-cell" pixel space.
pub struct Canvas {
    /// Width in pixels (sub-cell)
    pub width: usize,
    /// Height in pixels (sub-cell)
    pub height: usize,
    /// Pixel data: brightness 0.0..=1.0
    pub pixels: Vec<f64>,
    /// Per-pixel color (optional — used when color mode != Mono)
    pub colors: Vec<(u8, u8, u8)>,
    pub render_mode: RenderMode,
    pub color_mode: ColorMode,
    /// Optional per-cell character override (ASCII mode only).
    /// When set (non-\0), this char is used instead of brightness-mapped ASCII.
    pub char_override: Vec<char>,
    /// Color quantization step (0 = off, 4/8/16 = round RGB to nearest N).
    /// Higher values = fewer unique colors = better dedup = less output.
    pub color_quant: u8,
    /// Previous-frame brightness, used by temporal smoothing.
    /// NOT touched by `clear()` — persists across the per-frame wipe.
    /// `None` until first use; resets to `None` on `Canvas::new()`.
    pub prev_pixels: Option<Vec<f64>>,
}

impl Canvas {
    pub fn new(
        term_cols: usize,
        term_rows: usize,
        render_mode: RenderMode,
        color_mode: ColorMode,
    ) -> Self {
        let (px_w, px_h) = match render_mode {
            RenderMode::Braille => (term_cols * 2, term_rows * 4),
            RenderMode::HalfBlock => (term_cols, term_rows * 2),
            RenderMode::Ascii => (term_cols, term_rows),
        };
        let size = px_w * px_h;
        Canvas {
            width: px_w,
            height: px_h,
            pixels: vec![0.0; size],
            colors: vec![(255, 255, 255); size],
            char_override: vec!['\0'; size],
            render_mode,
            color_mode,
            color_quant: 0,
            prev_pixels: None,
        }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0.0);
        self.colors.fill((255, 255, 255));
        self.char_override.fill('\0');
    }

    /// Set a character directly at terminal-cell coordinates (ASCII mode).
    /// The character will be rendered as-is with the given color.
    #[inline]
    pub fn set_char(&mut self, x: usize, y: usize, ch: char, r: u8, g: u8, b: u8) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.char_override[idx] = ch;
            self.pixels[idx] = 1.0;
            self.colors[idx] = (r, g, b);
        }
    }

    /// Set a pixel (sub-cell coordinates). Bounds-checked.
    #[inline]
    #[allow(dead_code)]
    pub fn set(&mut self, x: usize, y: usize, brightness: f64) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = brightness;
        }
    }

    /// Set a pixel with color
    #[inline]
    pub fn set_colored(&mut self, x: usize, y: usize, brightness: f64, r: u8, g: u8, b: u8) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.pixels[idx] = brightness;
            self.colors[idx] = (r, g, b);
        }
    }

    /// Terminal dimensions needed for this canvas
    pub fn term_size(&self) -> (usize, usize) {
        match self.render_mode {
            RenderMode::Braille => (self.width / 2, self.height / 4),
            RenderMode::HalfBlock => (self.width, self.height / 2),
            RenderMode::Ascii => (self.width, self.height),
        }
    }

    /// Render the canvas to a string buffer for output
    pub fn render(&self) -> String {
        match self.render_mode {
            RenderMode::Braille => super::braille::render(self),
            RenderMode::HalfBlock => super::halfblock::render(self),
            RenderMode::Ascii => super::encoder::encode_full(&self.ascii_build_grid(), true),
        }
    }

    pub fn ascii_build_grid(&self) -> CellGrid {
        const CHARS: &[u8] = b" .:-=+*#%@";
        let cols = self.width;
        let rows = self.height;
        let use_color = self.color_mode != ColorMode::Mono;
        let mut cells = Vec::with_capacity(cols * rows);
        for row in 0..rows {
            for col in 0..cols {
                let idx = row * self.width + col;
                let v = self.pixels[idx].clamp(0.0, 1.0);
                let co = self.char_override[idx];
                let ch = if co != '\0' {
                    co
                } else {
                    CHARS[(v * (CHARS.len() - 1) as f64) as usize] as char
                };
                let fg = if use_color {
                    let (r, g, b) = self.colors[idx];
                    Some(self.map_color(r, g, b))
                } else {
                    None
                };
                cells.push(Cell { ch, fg, bg: None });
            }
        }
        CellGrid { cols, rows, cells }
    }

    /// Build the terminal-cell grid for the current render mode.
    pub fn build_grid(&self) -> CellGrid {
        match self.render_mode {
            RenderMode::Braille => super::braille::build_grid(self),
            RenderMode::HalfBlock => super::halfblock::build_grid(self),
            RenderMode::Ascii => self.ascii_build_grid(),
        }
    }

    /// Build the terminal-cell grid for dirty-cell diffing.
    pub fn render_cells(&self) -> super::cell::CellGrid {
        self.build_grid()
    }

    /// Blend each pixel's brightness toward its target using a first-order EMA.
    /// `alpha` in `(0.0, 1.0]`: `1.0` = no smoothing (no-op). Pass
    /// `smoothing_alpha(dt, tau)`. Brightness-only; `colors` is untouched.
    /// Uses and updates `prev_pixels`; snaps on first use / after a resize.
    pub fn apply_smoothing(&mut self, alpha: f64) {
        if alpha >= 1.0 {
            return;
        }
        let prev = self.prev_pixels.get_or_insert_with(|| self.pixels.clone());
        if prev.len() != self.pixels.len() {
            // Resize happened without Canvas::new() — re-snap to avoid bleed.
            *prev = self.pixels.clone();
            return;
        }
        for (p, pv) in self.pixels.iter_mut().zip(prev.iter_mut()) {
            let blended = *pv + alpha * (*p - *pv);
            *p = blended;
            *pv = blended;
        }
    }

    /// Apply post-processing effects to the canvas.
    /// `intensity`: brightness multiplier (1.0 = no change, 0.0 = black, 2.0 = double bright)
    /// `hue_shift`: hue rotation fraction (0.0 = no change, 0.5 = rotate 180°, 1.0 = full cycle)
    pub fn apply_effects(&mut self, intensity: f64, hue_shift: f64) {
        if (intensity - 1.0).abs() > 1e-10 {
            for p in &mut self.pixels {
                *p = (*p * intensity).clamp(0.0, 1.0);
            }
        }
        if hue_shift.abs() > 1e-10 {
            for c in &mut self.colors {
                *c = rotate_hue(*c, hue_shift);
            }
        }
    }

    /// Apply post-processing effects to the canvas.
    pub fn post_process(&mut self, config: &PostProcessConfig) {
        if config.bloom > 0.0 {
            self.apply_bloom(config.bloom, config.bloom_threshold);
        }
        if config.scanlines {
            self.apply_scanlines();
        }
        if config.vignette > 0.0 {
            self.apply_vignette(config.vignette);
        }
    }

    fn apply_bloom(&mut self, strength: f64, threshold: f64) {
        let w = self.width;
        let h = self.height;
        let mut brightened = vec![0.0f64; w * h];
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if self.pixels[idx] > threshold {
                    let boost = strength * 0.15 * self.pixels[idx];
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                                let nidx = ny as usize * w + nx as usize;
                                brightened[nidx] += boost;
                            }
                        }
                    }
                }
            }
        }
        for (pixel, boost) in self.pixels.iter_mut().zip(brightened.iter()) {
            *pixel = (*pixel + *boost).clamp(0.0, 1.0);
        }
    }

    fn apply_vignette(&mut self, strength: f64) {
        let cx = self.width as f64 / 2.0;
        let cy = self.height as f64 / 2.0;
        let max_dist = (cx * cx + cy * cy).sqrt();
        if max_dist < 1e-10 {
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt() / max_dist;
                let factor = 1.0 - (dist * dist * strength);
                let idx = y * self.width + x;
                self.pixels[idx] = (self.pixels[idx] * factor).clamp(0.0, 1.0);
            }
        }
    }

    fn apply_scanlines(&mut self) {
        for y in (0..self.height).step_by(2) {
            for x in 0..self.width {
                let idx = y * self.width + x;
                self.pixels[idx] *= 0.7;
            }
        }
    }

    pub fn map_color(&self, r: u8, g: u8, b: u8) -> Color {
        // Apply color quantization if enabled (reduces unique colors for better dedup)
        let (r, g, b) = if self.color_quant > 1 {
            let q = self.color_quant as u16;
            (
                ((r as u16 + q / 2) / q * q).min(255) as u8,
                ((g as u16 + q / 2) / q * q).min(255) as u8,
                ((b as u16 + q / 2) / q * q).min(255) as u8,
            )
        } else {
            (r, g, b)
        };
        match self.color_mode {
            ColorMode::Mono => Color::White,
            ColorMode::TrueColor => Color::Rgb { r, g, b },
            ColorMode::Ansi256 => {
                // Approximate RGB to 256-color
                let idx = 16 + (36 * (r as u16 / 51)) + (6 * (g as u16 / 51)) + (b as u16 / 51);
                Color::AnsiValue(idx as u8)
            }
            ColorMode::Ansi16 => {
                // Simple mapping to basic colors
                let brightness = (r as u16 + g as u16 + b as u16) / 3;
                if brightness < 64 {
                    Color::Black
                } else if r > g && r > b {
                    if brightness > 180 {
                        Color::Red
                    } else {
                        Color::DarkRed
                    }
                } else if g > r && g > b {
                    if brightness > 180 {
                        Color::Green
                    } else {
                        Color::DarkGreen
                    }
                } else if b > r && b > g {
                    if brightness > 180 {
                        Color::Blue
                    } else {
                        Color::DarkBlue
                    }
                } else if brightness > 180 {
                    Color::White
                } else {
                    Color::Grey
                }
            }
        }
    }
}

fn rotate_hue(rgb: (u8, u8, u8), shift: f64) -> (u8, u8, u8) {
    let (r, g, b) = rgb;
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta < 1e-10 {
        0.0
    } else if (max - r).abs() < 1e-10 {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if (max - g).abs() < 1e-10 {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = (h + shift * 360.0).rem_euclid(360.0);
    let s = if max < 1e-10 { 0.0 } else { delta / max };
    let v = max;

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

pub fn color_to_fg(color: Color) -> String {
    match color {
        Color::Rgb { r, g, b } => format!("38;2;{};{};{}", r, g, b),
        Color::AnsiValue(v) => format!("38;5;{}", v),
        Color::Black => "30".into(),
        Color::DarkRed => "31".into(),
        Color::DarkGreen => "32".into(),
        Color::DarkYellow => "33".into(),
        Color::DarkBlue => "34".into(),
        Color::DarkMagenta => "35".into(),
        Color::DarkCyan => "36".into(),
        Color::Grey => "37".into(),
        Color::DarkGrey => "90".into(),
        Color::Red => "91".into(),
        Color::Green => "92".into(),
        Color::Yellow => "93".into(),
        Color::Blue => "94".into(),
        Color::Magenta => "95".into(),
        Color::Cyan => "96".into(),
        Color::White => "97".into(),
        _ => "37".into(),
    }
}

/// EMA smoothing factor for a frame interval `dt` and time constant `tau`
/// (both in seconds): `alpha = 1 - exp(-dt / tau)`. Equals `1 - 1/e ≈ 0.632`
/// when `dt == tau`. Callers must guard `tau > 0` (this divides by `tau`).
pub fn smoothing_alpha(dt: f64, tau: f64) -> f64 {
    1.0 - (-dt / tau).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_canvas() -> Canvas {
        Canvas::new(10, 10, RenderMode::HalfBlock, ColorMode::TrueColor)
    }

    #[test]
    fn test_set_and_get_pixel() {
        let mut c = test_canvas();
        c.set(5, 5, 0.75);
        assert!((c.pixels[5 * 10 + 5] - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_out_of_bounds_is_noop() {
        let mut c = test_canvas();
        // Should not panic
        c.set(100, 100, 1.0);
        c.set(usize::MAX, usize::MAX, 1.0);
    }

    #[test]
    fn test_clear_zeroes_all_pixels() {
        let mut c = test_canvas();
        c.set(5, 5, 1.0);
        c.clear();
        assert!(c.pixels.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_set_colored_stores_color() {
        let mut c = test_canvas();
        c.set_colored(3, 3, 0.5, 255, 128, 0);
        let idx = 3 * 10 + 3;
        assert_eq!(c.colors[idx], (255, 128, 0));
        assert!((c.pixels[idx] - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bloom_brightens_neighbors_of_bright_pixel() {
        let mut c = test_canvas();
        let cx = 5;
        let cy = 5;
        c.pixels[cy * c.width + cx] = 0.9;
        let cfg = PostProcessConfig {
            bloom: 0.5,
            bloom_threshold: 0.6,
            vignette: 0.0,
            scanlines: false,
        };
        c.post_process(&cfg);
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (cx as i32 + dx) as usize;
                let ny = (cy as i32 + dy) as usize;
                if nx < c.width && ny < c.height {
                    assert!(c.pixels[ny * c.width + nx] > 0.0);
                }
            }
        }
    }

    #[test]
    fn test_vignette_darkens_edges() {
        let mut c = Canvas::new(10, 10, RenderMode::HalfBlock, ColorMode::TrueColor);
        for p in &mut c.pixels {
            *p = 1.0;
        }
        let cfg = PostProcessConfig {
            bloom: 0.0,
            bloom_threshold: 0.6,
            vignette: 0.8,
            scanlines: false,
        };
        c.post_process(&cfg);
        let center = c.pixels[5 * c.width + 5];
        let corner = c.pixels[0];
        assert!(corner < center);
    }

    #[test]
    fn test_scanlines_darkens_even_rows() {
        let mut c = test_canvas();
        for p in &mut c.pixels {
            *p = 1.0;
        }
        let cfg = PostProcessConfig {
            bloom: 0.0,
            bloom_threshold: 0.6,
            vignette: 0.0,
            scanlines: true,
        };
        c.post_process(&cfg);
        let even_val = c.pixels[0];
        let odd_val = c.pixels[c.width];
        assert!(even_val < odd_val);
        assert!((odd_val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_post_process_noop_when_all_disabled() {
        let mut c = test_canvas();
        c.pixels[5 * c.width + 5] = 0.75;
        let before = c.pixels[5 * c.width + 5];
        c.post_process(&PostProcessConfig::default());
        assert!((c.pixels[5 * c.width + 5] - before).abs() < 1e-10);
    }

    #[test]
    fn smoothing_alpha_at_tau_equals_one_minus_inv_e() {
        // dt == tau => alpha = 1 - e^-1 ≈ 0.6321
        let alpha = super::smoothing_alpha(0.1, 0.1);
        assert!((alpha - (1.0 - 1.0 / std::f64::consts::E)).abs() < 1e-9);
    }

    #[test]
    fn smoothing_alpha_grows_with_dt_and_stays_in_unit_interval() {
        let small = super::smoothing_alpha(0.01, 0.1);
        let large = super::smoothing_alpha(0.2, 0.1);
        assert!(small > 0.0 && small < 1.0);
        assert!(large > 0.0 && large < 1.0);
        assert!(small < large, "larger dt => larger alpha (snaps faster)");
    }

    #[test]
    fn new_initializes_prev_pixels_none() {
        let c = Canvas::new(4, 2, RenderMode::HalfBlock, ColorMode::TrueColor);
        assert!(c.prev_pixels.is_none());
    }

    #[test]
    fn clear_does_not_reset_prev_pixels() {
        let mut c = Canvas::new(4, 2, RenderMode::HalfBlock, ColorMode::TrueColor);
        c.prev_pixels = Some(vec![0.5; c.pixels.len()]);
        c.clear();
        let prev = c.prev_pixels.expect("prev_pixels must survive clear()");
        assert!(prev.iter().all(|&v| (v - 0.5).abs() < 1e-12));
    }

    #[test]
    fn apply_smoothing_first_call_snaps_to_target() {
        let mut c = Canvas::new(4, 1, RenderMode::HalfBlock, ColorMode::TrueColor);
        c.pixels[0] = 1.0; // target = full bright
        c.apply_smoothing(0.5);
        // prev was None -> snap: pixel unchanged, prev now equals pixels
        assert!((c.pixels[0] - 1.0).abs() < 1e-12);
        assert!((c.prev_pixels.as_ref().unwrap()[0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn apply_smoothing_half_step_blends_and_updates_prev() {
        let mut c = Canvas::new(4, 1, RenderMode::HalfBlock, ColorMode::TrueColor);
        c.prev_pixels = Some(vec![0.0; c.pixels.len()]); // prev = black
        c.pixels[0] = 1.0; // target = full bright
        c.apply_smoothing(0.5);
        assert!((c.pixels[0] - 0.5).abs() < 1e-12);
        assert!((c.prev_pixels.as_ref().unwrap()[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn apply_smoothing_held_step_converges_monotonically() {
        let mut c = Canvas::new(2, 1, RenderMode::HalfBlock, ColorMode::TrueColor);
        c.prev_pixels = Some(vec![0.0; c.pixels.len()]);
        let mut last = 0.0f64;
        for _ in 0..40 {
            c.pixels[0] = 1.0; // held target
            c.apply_smoothing(0.3);
            assert!(c.pixels[0] > last, "monotonic increase");
            assert!(c.pixels[0] <= 1.0, "bounded by 1.0");
            last = c.pixels[0];
        }
        assert!(last > 0.99, "converges toward the target");
    }

    #[test]
    fn apply_smoothing_alpha_ge_one_is_noop() {
        let mut c = Canvas::new(2, 1, RenderMode::HalfBlock, ColorMode::TrueColor);
        c.prev_pixels = Some(vec![0.0; c.pixels.len()]);
        c.pixels[0] = 0.8;
        c.apply_smoothing(1.0);
        assert!(
            (c.pixels[0] - 0.8).abs() < 1e-12,
            "alpha >= 1.0 must leave pixels unchanged"
        );
    }

    /// Deterministic canvases exercising each render mode × color mode + a bloom variant.
    /// KEEP STABLE — their rendered bytes are the golden snapshots.
    fn snapshot_fixtures() -> Vec<(String, Canvas)> {
        let mut v: Vec<(String, Canvas)> = Vec::new();
        for mode in [
            RenderMode::HalfBlock,
            RenderMode::Braille,
            RenderMode::Ascii,
        ] {
            for cm in [ColorMode::Mono, ColorMode::TrueColor, ColorMode::Ansi256] {
                let name = format!("{mode:?}-{cm:?}").to_lowercase().replace(' ', "");
                let mut c = Canvas::new(8, 4, mode, cm);
                for i in 0..16usize {
                    let x = i % c.width;
                    let y = i / c.width;
                    if x < c.width && y < c.height {
                        c.set_colored(x, y, 0.8, 200 - i as u8 * 3, i as u8 * 7, 100);
                    }
                }
                v.push((name, c));
            }
        }
        let mut c = Canvas::new(8, 4, RenderMode::HalfBlock, ColorMode::TrueColor);
        for i in 0..16usize {
            let x = i % c.width;
            let y = i / c.width;
            if x < c.width && y < c.height {
                c.set_colored(x, y, 0.9, 255, 100, 50);
            }
        }
        c.apply_effects(1.0, 0.0);
        c.post_process(&PostProcessConfig {
            bloom: 0.5,
            bloom_threshold: 0.6,
            vignette: 0.4,
            scanlines: false,
        });
        v.push(("halfblock-truecolor-bloom".to_string(), c));
        v
    }

    fn snapshot_dir() -> String {
        format!("{}/src/render/snapshots", env!("CARGO_MANIFEST_DIR"))
    }

    /// Regenerate the snapshot fixture files from the CURRENT render() output.
    /// Run: cargo test gen_render_snapshots -- --ignored --nocapture
    #[test]
    #[ignore = "regenerator; run with --ignored"]
    fn gen_render_snapshots() {
        let dir = snapshot_dir();
        std::fs::create_dir_all(&dir).unwrap();
        for (name, c) in snapshot_fixtures() {
            std::fs::write(format!("{dir}/{name}.txt"), c.render()).unwrap();
        }
        eprintln!("wrote snapshots to {dir}");
    }

    #[test]
    fn build_grid_halfblock_colored_and_dark() {
        // col 0: top bright, bottom dark-but-present → ▀ with fg + bg(Some dark color)
        // col 1: both dark → space cell with fg/bg None
        let mut c = Canvas::new(2, 1, RenderMode::HalfBlock, ColorMode::TrueColor);
        c.set_colored(0, 0, 1.0, 255, 0, 0); // top bright
        c.set_colored(0, 1, 0.0, 255, 0, 0); // bottom dark
        let g = c.build_grid();
        let cell = g.get(0, 0);
        assert_eq!(cell.ch, '▀');
        assert!(cell.fg.is_some(), "bright top → fg Some");
        assert!(
            cell.bg.is_some(),
            "dark-but-present bottom → bg Some (mirrors render())"
        );
        let dark = g.get(0, 1);
        assert_eq!(dark.ch, ' ', "both dark → space");
        assert!(dark.fg.is_none() && dark.bg.is_none(), "both dark → no SGR");
    }

    #[test]
    fn render_matches_snapshots() {
        let dir = snapshot_dir();
        for (name, c) in snapshot_fixtures() {
            let path = format!("{dir}/{name}.txt");
            let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "missing snapshot {path}: run `cargo test gen_render_snapshots -- --ignored` ({e})"
                )
            });
            assert_eq!(
                c.render(),
                expected,
                "render() bytes changed for fixture {name}"
            );
        }
    }
}
