use crate::render::Canvas;
use super::Animation;

/// Slowly zooming into the Mandelbrot set with color cycling
pub struct Mandelbrot {
    zoom: f64,
    target_x: f64,
    target_y: f64,
}

impl Mandelbrot {
    pub fn new() -> Self {
        Mandelbrot {
            zoom: 1.0,
            // Zoom target: a visually interesting point near the boundary
            target_x: -0.7436,
            target_y: 0.1319,
        }
    }
}

impl Animation for Mandelbrot {
    fn name(&self) -> &str {
        "mandelbrot"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let max_iter = 80;

        // Smooth zoom cycle
        let zoom_cycle = (time * 0.1).sin() * 0.5 + 0.5; // 0 to 1
        self.zoom = (1.0 + zoom_cycle * 12.0).exp(); // exponential zoom
        let color_offset = time * 0.3;

        let scale = 3.0 / self.zoom;
        let aspect = w / h;

        canvas.clear();

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64 / w;
                let fy = y as f64 / h;

                // Map pixel to complex plane
                let cr = self.target_x + (fx - 0.5) * scale * aspect;
                let ci = self.target_y + (fy - 0.5) * scale;

                // Mandelbrot iteration
                let mut zr = 0.0;
                let mut zi = 0.0;
                let mut iter = 0;

                while iter < max_iter {
                    let zr2 = zr * zr;
                    let zi2 = zi * zi;
                    if zr2 + zi2 > 4.0 {
                        break;
                    }
                    zi = 2.0 * zr * zi + ci;
                    zr = zr2 - zi2 + cr;
                    iter += 1;
                }

                if iter < max_iter {
                    // Smooth coloring using escape-time with continuous iteration count
                    let zr2 = zr * zr;
                    let zi2 = zi * zi;
                    let log_zn = (zr2 + zi2).ln() * 0.5;
                    let nu = (log_zn / 2.0_f64.ln()).ln() / 2.0_f64.ln();
                    let smooth_iter = iter as f64 + 1.0 - nu;

                    let t = smooth_iter / max_iter as f64;
                    let hue = (t * 3.0 + color_offset).fract();
                    let saturation = 0.8;
                    let value = (1.0 - t * 0.3).clamp(0.5, 1.0);

                    let (r, g, b) = hsv_to_rgb(hue, saturation, value);
                    let brightness = value * 0.9;
                    canvas.set_colored(x, y, brightness, r, g, b);
                }
                // Points inside the set stay black (cleared)
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
