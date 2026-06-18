use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Drop {
    x: usize,
    y: f64,
    speed: f64,
    length: usize,
}

struct Layer {
    drops: Vec<Drop>,
    speed_min: f64,
    speed_max: f64,
    head_r: u8,
    head_g: u8,
    head_b: u8,
    trail_g_base: u8,
    trail_g_range: u8,
}

impl Layer {
    #[allow(clippy::too_many_arguments)]
    fn create_drops(
        rng: &mut rand::rngs::ThreadRng,
        count: usize,
        width: usize,
        height: usize,
        length_min: usize,
        length_max: usize,
        speed_min: f64,
        speed_max: f64,
    ) -> Vec<Drop> {
        (0..count)
            .map(|_| Drop {
                x: rng.random_range(0..width),
                y: rng.random_range(0.0..height as f64),
                speed: rng.random_range(speed_min..speed_max),
                length: rng.random_range(length_min..length_max),
            })
            .collect()
    }
}

/// A uniformly-random glyph for the digital rain, drawn from hiragana (U+3041–U+3096),
/// katakana (U+30A1–U+30FA), ASCII `A–Z` / `a–z`, and digits `0–9`. Kana dominate by range
/// size, with Latin/digits sprinkled in for the classic mixed Matrix-code look.
fn random_glyph(rng: &mut rand::rngs::ThreadRng) -> char {
    const RANGES: &[(u32, u32)] = &[
        (0x3041, 0x3096), // hiragana (86)
        (0x30A1, 0x30FA), // katakana (90)
        (0x41, 0x5A),     // A–Z (26)
        (0x61, 0x7A),     // a–z (26)
        (0x30, 0x39),     // 0–9 (10)
    ];
    let total: u32 = RANGES.iter().map(|&(a, b)| b - a + 1).sum();
    let mut n = rng.random_range(0..total);
    for &(start, end) in RANGES {
        let len = end - start + 1;
        if n < len {
            return char::from_u32(start + n).unwrap_or('ア');
        }
        n -= len;
    }
    unreachable!("glyph ranges are exhaustive over `total`")
}

fn draw_layer(
    canvas: &mut Canvas,
    layer: &mut Layer,
    rng: &mut rand::rngs::ThreadRng,
    width: usize,
    height: usize,
    dt: f64,
    len_range: (usize, usize),
) {
    let speed_min = layer.speed_min;
    let speed_max = layer.speed_max;
    let trail_g_base = layer.trail_g_base;
    let trail_g_range = layer.trail_g_range;
    let head_r = layer.head_r;
    let head_g = layer.head_g;
    let head_b = layer.head_b;

    for drop in &mut layer.drops {
        drop.y += drop.speed * dt;

        if drop.y as usize > height + drop.length {
            drop.y = -(drop.length as f64);
            drop.x = rng.random_range(0..width);
            drop.speed = rng.random_range(speed_min..speed_max);
            drop.length = rng.random_range(len_range.0..len_range.1);
        }

        // A wide glyph occupies 2 columns, so never draw in the last column (it would
        // overflow the right edge); that column is left dark instead.
        let draw = drop.x + 1 < canvas.width;
        let head = drop.y as isize;
        for i in 0..drop.length {
            let py = head - i as isize;
            if draw && py >= 0 && (py as usize) < canvas.height {
                let fade = 1.0 - (i as f64 / drop.length as f64);
                let g = trail_g_base + ((trail_g_range as f64 * fade) as u8);
                canvas.set_char(drop.x, py as usize, random_glyph(rng), 0, g, 0);
            }
        }

        // Bright head glyph
        if draw && head >= 0 && (head as usize) < canvas.height {
            canvas.set_char(
                drop.x,
                head as usize,
                random_glyph(rng),
                head_r,
                head_g,
                head_b,
            );
        }
    }
}

/// Matrix digital rain with multi-layer depth
pub struct Matrix {
    width: usize,
    height: usize,
    scale: f64,
    far: Layer,
    mid: Layer,
    near: Layer,
    // length ranges stored for resets
    far_len: (usize, usize),
    mid_len: (usize, usize),
    near_len: (usize, usize),
    rng: rand::rngs::ThreadRng,
}

impl Matrix {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();

        let far_count = ((width as f64 * 0.8) * scale) as usize;
        let mid_count = ((width as f64 * 0.5) * scale) as usize;
        let near_count = ((width as f64 * 0.25) * scale) as usize;

        let far_len = (3, 5.min(height / 2));
        let mid_len = (5, 8.min(height / 2));
        let near_len = (8, 12.min(height / 2));

        let far = Layer {
            drops: Layer::create_drops(
                &mut rng, far_count, width, height, far_len.0, far_len.1, 2.0, 8.0,
            ),
            speed_min: 2.0,
            speed_max: 8.0,
            head_r: 0,
            head_g: 100,
            head_b: 0,
            trail_g_base: 40,
            trail_g_range: 60,
        };

        let mid = Layer {
            drops: Layer::create_drops(
                &mut rng, mid_count, width, height, mid_len.0, mid_len.1, 6.0, 16.0,
            ),
            speed_min: 6.0,
            speed_max: 16.0,
            head_r: 0,
            head_g: 200,
            head_b: 0,
            trail_g_base: 80,
            trail_g_range: 120,
        };

        let near = Layer {
            drops: Layer::create_drops(
                &mut rng, near_count, width, height, near_len.0, near_len.1, 10.0, 24.0,
            ),
            speed_min: 10.0,
            speed_max: 24.0,
            head_r: 200,
            head_g: 255,
            head_b: 200,
            trail_g_base: 100,
            trail_g_range: 155,
        };

        Matrix {
            width,
            height,
            scale,
            far,
            mid,
            near,
            far_len,
            mid_len,
            near_len,
            rng: rand::rng(),
        }
    }
}

impl Animation for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Ascii
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        canvas.clear();

        let width = self.width;
        let height = self.height;
        let far_len = self.far_len;
        let mid_len = self.mid_len;
        let near_len = self.near_len;

        // Draw order: far (background) -> mid -> near (foreground)
        draw_layer(
            canvas,
            &mut self.far,
            &mut self.rng,
            width,
            height,
            dt,
            far_len,
        );
        draw_layer(
            canvas,
            &mut self.mid,
            &mut self.rng,
            width,
            height,
            dt,
            mid_len,
        );
        draw_layer(
            canvas,
            &mut self.near,
            &mut self.rng,
            width,
            height,
            dt,
            near_len,
        );
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        let far_count = ((width as f64 * 0.8) * self.scale) as usize;
        let mid_count = ((width as f64 * 0.5) * self.scale) as usize;
        let near_count = ((width as f64 * 0.25) * self.scale) as usize;

        self.far_len = (3, 5.min(height / 2));
        self.mid_len = (5, 8.min(height / 2));
        self.near_len = (8, 12.min(height / 2));

        self.far.drops = Layer::create_drops(
            &mut self.rng,
            far_count,
            width,
            height,
            self.far_len.0,
            self.far_len.1,
            self.far.speed_min,
            self.far.speed_max,
        );
        self.mid.drops = Layer::create_drops(
            &mut self.rng,
            mid_count,
            width,
            height,
            self.mid_len.0,
            self.mid_len.1,
            self.mid.speed_min,
            self.mid.speed_max,
        );
        self.near.drops = Layer::create_drops(
            &mut self.rng,
            near_count,
            width,
            height,
            self.near_len.0,
            self.near_len.1,
            self.near.speed_min,
            self.near.speed_max,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Canvas, ColorMode, RenderMode};

    #[test]
    fn matrix_renders_kana_glyphs() {
        // Drive the real Ascii render path (ascii_build_grid -> encode_full) and confirm
        // hiragana/katakana glyphs actually appear in the encoded output.
        let mut canvas = Canvas::new(80, 25, RenderMode::Ascii, ColorMode::TrueColor);
        let mut anim = Matrix::new(canvas.width, canvas.height, 1.0);
        anim.on_resize(canvas.width, canvas.height);
        for f in 0..20u32 {
            anim.update(&mut canvas, 1.0 / 24.0, f as f64 / 24.0);
        }
        let out = canvas.render();
        let has_kana = out.chars().any(|c| ('\u{3040}'..='\u{30FF}').contains(&c));
        assert!(
            has_kana,
            "matrix output should contain hiragana/katakana glyphs"
        );
    }
}
