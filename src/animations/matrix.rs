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
    brightness_min: f64,
    brightness_max: f64,
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
    let brightness_min = layer.brightness_min;
    let brightness_max = layer.brightness_max;
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

        let head = drop.y as isize;
        for i in 0..drop.length {
            let py = head - i as isize;
            if py >= 0 && (py as usize) < canvas.height && drop.x < canvas.width {
                let fade = 1.0 - (i as f64 / drop.length as f64);
                let brightness = (brightness_min + (brightness_max - brightness_min) * fade * fade)
                    .min(brightness_max);
                let g = trail_g_base + ((trail_g_range as f64 * fade) as u8);
                canvas.set_colored(drop.x, py as usize, brightness, 0, g, 0);
            }
        }

        // Bright head character
        if head >= 0 && (head as usize) < canvas.height && drop.x < canvas.width {
            let head_brightness = brightness_max;
            canvas.set_colored(
                drop.x,
                head as usize,
                head_brightness,
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
            brightness_min: 0.2,
            brightness_max: 0.4,
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
            brightness_min: 0.4,
            brightness_max: 0.7,
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
            brightness_min: 0.7,
            brightness_max: 1.0,
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
