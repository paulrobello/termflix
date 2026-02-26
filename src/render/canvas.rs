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
            RenderMode::Ascii => self.render_ascii(),
        }
    }

    fn render_ascii(&self) -> String {
        const CHARS: &[u8] = b" .:-=+*#%@";
        let (cols, rows) = self.term_size();
        let mut out = String::with_capacity(cols * rows * 10);
        let use_color = self.color_mode != ColorMode::Mono;
        let mut last_fg = String::new();

        for row in 0..rows {
            for col in 0..cols {
                let idx = row * self.width + col;
                let v = self.pixels[idx].clamp(0.0, 1.0);
                let co = self.char_override[idx];
                let ch = if co != '\0' {
                    co
                } else {
                    let ci = (v * (CHARS.len() - 1) as f64) as usize;
                    CHARS[ci] as char
                };

                if use_color {
                    let (r, g, b) = self.colors[idx];
                    let color = self.map_color(r, g, b);
                    let fg = color_to_fg(color);
                    if fg != last_fg {
                        out.push_str("\x1b[");
                        out.push_str(&fg);
                        out.push('m');
                        last_fg = fg;
                    }
                }
                out.push(ch);
            }
            out.push_str("\x1b[0m\x1b[");
            let next_row = row + 2;
            out.push_str(&next_row.to_string());
            out.push_str(";1H");
            last_fg.clear();
        }
        out
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
