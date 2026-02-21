use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Fake audio spectrum analyzer bars bouncing to imaginary music
pub struct Visualizer {
    bars: Vec<f64>,
    targets: Vec<f64>,
    peaks: Vec<f64>,
    peak_vel: Vec<f64>,
    beat_timer: f64,
    beat_interval: f64,
    energy: f64,
}

impl Visualizer {
    pub fn new(width: usize, _height: usize, _scale: f64) -> Self {
        let bar_count = (width / 2).max(8);
        Visualizer {
            bars: vec![0.0; bar_count],
            targets: vec![0.0; bar_count],
            peaks: vec![0.0; bar_count],
            peak_vel: vec![0.0; bar_count],
            beat_timer: 0.0,
            beat_interval: 0.5,
            energy: 0.5,
        }
    }
}

impl Animation for Visualizer {
    fn name(&self) -> &str {
        "visualizer"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let mut rng = rand::rng();
        let w = canvas.width;
        let h = canvas.height;

        // Resize bars if needed
        let bar_count = (w / 2).max(8);
        self.bars.resize(bar_count, 0.0);
        self.targets.resize(bar_count, 0.0);
        self.peaks.resize(bar_count, 0.0);
        self.peak_vel.resize(bar_count, 0.0);

        // Simulate music beats
        self.beat_timer -= dt;
        if self.beat_timer <= 0.0 {
            self.beat_interval = rng.random_range(0.3..0.8);
            self.beat_timer = self.beat_interval;
            self.energy = rng.random_range(0.3..1.0);

            // Set new targets for each bar (frequency spectrum shape)
            for i in 0..bar_count {
                let freq = i as f64 / bar_count as f64;
                // Bass-heavy with occasional treble
                let bass = (1.0 - freq).powi(2) * self.energy;
                let mid = (-(freq - 0.4).powi(2) * 10.0).exp() * self.energy * 0.7;
                let treble = freq.powi(3) * rng.random_range(0.0..self.energy * 0.5);
                self.targets[i] =
                    (bass + mid + treble + rng.random_range(0.0..0.2)).clamp(0.0, 1.0);
            }
        }

        // Animate bars toward targets
        for i in 0..bar_count {
            let diff = self.targets[i] - self.bars[i];
            if diff > 0.0 {
                self.bars[i] += diff * dt * 12.0; // Fast attack
            } else {
                self.bars[i] += diff * dt * 4.0; // Slow decay
            }
            self.bars[i] = self.bars[i].clamp(0.0, 1.0);

            // Peak indicator with gravity
            if self.bars[i] > self.peaks[i] {
                self.peaks[i] = self.bars[i];
                self.peak_vel[i] = 0.0;
            } else {
                self.peak_vel[i] += dt * 1.5; // gravity
                self.peaks[i] -= self.peak_vel[i] * dt;
                if self.peaks[i] < 0.0 {
                    self.peaks[i] = 0.0;
                }
            }
        }

        canvas.clear();

        // Draw bars
        let bar_width = (w / bar_count).max(1);
        let gap = if bar_width > 1 { 1 } else { 0 };

        for i in 0..bar_count {
            let bar_height = (self.bars[i] * h as f64) as usize;
            let bar_x = i * bar_width;

            for dy in 0..bar_height {
                let y = h.saturating_sub(1 + dy);
                let frac = dy as f64 / h as f64;

                // Color gradient: green -> yellow -> red from bottom to top
                let (r, g, b) = bar_color(frac, time, i as f64 / bar_count as f64);

                for bx in 0..(bar_width.saturating_sub(gap)) {
                    let px = bar_x + bx;
                    if px < canvas.width && y < canvas.height {
                        canvas.set_colored(px, y, 0.7 + frac * 0.3, r, g, b);
                    }
                }
            }

            // Peak indicator
            let peak_y = h.saturating_sub(1 + (self.peaks[i] * h as f64) as usize);
            for bx in 0..(bar_width.saturating_sub(gap)) {
                let px = bar_x + bx;
                if px < canvas.width && peak_y < canvas.height {
                    canvas.set_colored(px, peak_y, 1.0, 255, 255, 255);
                }
            }
        }

        // Reflection at bottom (subtle)
        let reflect_h = (h / 6).min(5);
        for i in 0..bar_count {
            let bar_height = (self.bars[i] * h as f64) as usize;
            let bar_x = i * bar_width;

            for dy in 0..reflect_h.min(bar_height) {
                let y = h.saturating_sub(1).wrapping_sub(0).min(canvas.height - 1);
                let src_y = h.saturating_sub(1 + dy);
                if src_y >= canvas.height || y >= canvas.height {
                    continue;
                }
                let fade = 0.15 * (1.0 - dy as f64 / reflect_h as f64);

                for bx in 0..(bar_width.saturating_sub(gap)) {
                    let px = bar_x + bx;
                    if px < canvas.width {
                        let frac = dy as f64 / h as f64;
                        let (r, g, b) = bar_color(frac, time, i as f64 / bar_count as f64);
                        // Reflection goes below the bottom
                        let ry = (h - 1).wrapping_add(dy + 1);
                        if ry < canvas.height {
                            canvas.set_colored(px, ry, fade, r / 3, g / 3, b / 3);
                        }
                    }
                }
            }
        }
    }
}

fn bar_color(height_frac: f64, _time: f64, _bar_pos: f64) -> (u8, u8, u8) {
    if height_frac > 0.8 {
        // Red zone
        (255, (60.0 * (1.0 - (height_frac - 0.8) / 0.2)) as u8, 0)
    } else if height_frac > 0.5 {
        // Yellow zone
        let f = (height_frac - 0.5) / 0.3;
        ((100.0 + 155.0 * f) as u8, (255.0 - 50.0 * f) as u8, 0)
    } else {
        // Green zone
        let f = height_frac / 0.5;
        (0, (120.0 + 135.0 * f) as u8, (50.0 * (1.0 - f)) as u8)
    }
}
