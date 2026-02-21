use crate::render::Canvas;
use super::Animation;

/// Dragon curve fractal iteratively drawn and colored
pub struct Dragon {
    points: Vec<(f64, f64)>,
    iteration: usize,
    max_iteration: usize,
    rebuild_timer: f64,
    draw_progress: f64,
}

impl Dragon {
    pub fn new() -> Self {
        Dragon {
            points: Vec::new(),
            iteration: 0,
            max_iteration: 14,
            rebuild_timer: 0.0,
            draw_progress: 0.0,
        }
    }

    fn build_curve(&mut self, iteration: usize) {
        // Start with a simple line
        self.points.clear();
        self.points.push((0.0, 0.0));
        self.points.push((1.0, 0.0));

        for _ in 0..iteration {
            let mut new_points = Vec::with_capacity(self.points.len() * 2);
            let last = self.points.last().copied().unwrap_or((0.0, 0.0));
            let pivot_x = last.0;
            let pivot_y = last.1;

            // Keep original points
            for &p in &self.points {
                new_points.push(p);
            }

            // Add rotated copy (90 degrees around the last point, reversed)
            for i in (0..self.points.len() - 1).rev() {
                let p = self.points[i];
                let dx = p.0 - pivot_x;
                let dy = p.1 - pivot_y;
                // Rotate 90 degrees counterclockwise
                let rx = -dy + pivot_x;
                let ry = dx + pivot_y;
                new_points.push((rx, ry));
            }

            self.points = new_points;
        }
    }
}

impl Animation for Dragon {
    fn name(&self) -> &str {
        "dragon"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;

        // Cycle through iterations
        self.rebuild_timer += dt;
        let target_iter = ((time * 0.5) as usize % (self.max_iteration + 1)).max(3);

        if target_iter != self.iteration || self.points.is_empty() {
            self.iteration = target_iter;
            self.build_curve(self.iteration);
            self.draw_progress = 0.0;
        }

        // Animate drawing progress
        self.draw_progress = (self.draw_progress + dt * 0.8).min(1.0);

        canvas.clear();

        if self.points.len() < 2 {
            return;
        }

        // Find bounds for scaling
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for &(px, py) in &self.points {
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_y = min_y.min(py);
            max_y = max_y.max(py);
        }

        let range_x = (max_x - min_x).max(0.001);
        let range_y = (max_y - min_y).max(0.001);
        let scale = (w * 0.8 / range_x).min(h * 0.8 / range_y);
        let offset_x = (w - range_x * scale) * 0.5 - min_x * scale;
        let offset_y = (h - range_y * scale) * 0.5 - min_y * scale;

        // Draw the curve progressively
        let points_to_draw = ((self.points.len() as f64 * self.draw_progress) as usize)
            .min(self.points.len());

        for i in 0..points_to_draw.saturating_sub(1) {
            let (x0, y0) = self.points[i];
            let (x1, y1) = self.points[i + 1];

            let sx0 = x0 * scale + offset_x;
            let sy0 = y0 * scale + offset_y;
            let sx1 = x1 * scale + offset_x;
            let sy1 = y1 * scale + offset_y;

            // Color based on position in curve
            let t = i as f64 / self.points.len() as f64;
            let hue = (t + time * 0.1).fract();
            let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);

            // Draw line segment using Bresenham-like stepping
            let steps = ((sx1 - sx0).abs().max((sy1 - sy0).abs()) as usize).max(1);
            for s in 0..=steps {
                let frac = s as f64 / steps as f64;
                let px = (sx0 + (sx1 - sx0) * frac) as usize;
                let py = (sy0 + (sy1 - sy0) * frac) as usize;
                if px < canvas.width && py < canvas.height {
                    canvas.set_colored(px, py, 0.9, r, g, b);
                }
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
