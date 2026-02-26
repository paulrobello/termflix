use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Bolt {
    segments: Vec<(f64, f64, f64, f64)>, // (x1, y1, x2, y2)
    brightness: f64,
    life: f64,
}

/// Lightning bolts with branching
pub struct Lightning {
    width: usize,
    height: usize,
    bolts: Vec<Bolt>,
    spawn_timer: f64,
    flash: f64,
    rng: rand::rngs::ThreadRng,
}

impl Lightning {
    pub fn new(width: usize, height: usize) -> Self {
        Lightning {
            width,
            height,
            bolts: Vec::new(),
            spawn_timer: 0.0,
            flash: 0.0,
            rng: rand::rng(),
        }
    }

    fn generate_bolt(&mut self) -> Bolt {
        let w = self.width;
        let h = self.height;
        let start_x = self.rng.random_range(w as f64 * 0.1..w as f64 * 0.9);
        let mut segments = Vec::new();

        Self::branch_static(
            &mut segments,
            start_x,
            0.0,
            h as f64 * 0.8,
            0,
            w,
            h,
            &mut self.rng,
        );

        Bolt {
            segments,
            brightness: 1.0,
            life: self.rng.random_range(0.15..0.4),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn branch_static(
        segments: &mut Vec<(f64, f64, f64, f64)>,
        x: f64,
        y: f64,
        target_y: f64,
        depth: u32,
        _width: usize,
        height: usize,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        if depth > 5 || y > target_y {
            return;
        }

        let mut cx = x;
        let mut cy = y;
        let step = height as f64 * 0.05;

        while cy < target_y {
            let nx = cx + rng.random_range(-step * 1.5..step * 1.5);
            let ny = cy + rng.random_range(step * 0.5..step * 1.5);
            segments.push((cx, cy, nx, ny));

            // Random branching
            if depth < 3 && rng.random_range(0.0..1.0) < 0.15 {
                let branch_target = ny + rng.random_range(step * 2.0..step * 5.0);
                let branch_x = nx + rng.random_range(-step * 3.0..step * 3.0);
                Self::branch_static(
                    segments,
                    nx,
                    ny,
                    branch_target.min(target_y),
                    depth + 1,
                    _width,
                    height,
                    rng,
                );
                // Continue the main bolt slightly offset
                let _ = branch_x; // branch direction already applied via recursive call
            }

            cx = nx;
            cy = ny;
        }
    }
}

impl Animation for Lightning {
    fn name(&self) -> &str {
        "lightning"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;

        // Spawn new bolts
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            let bolt = self.generate_bolt();
            self.bolts.push(bolt);
            self.flash = 0.3;
            // Random interval between bolts
            self.spawn_timer = self.rng.random_range(0.5..3.0);
        }

        // Fade flash
        self.flash = (self.flash - dt * 2.0).max(0.0);

        canvas.clear();

        // Draw background flash
        if self.flash > 0.0 {
            let flash_brightness = self.flash * 0.15;
            for y in 0..canvas.height {
                for x in 0..canvas.width {
                    canvas.set_colored(x, y, flash_brightness, 100, 100, 130);
                }
            }
        }

        // Update and draw bolts
        for bolt in &mut self.bolts {
            bolt.life -= dt;
            bolt.brightness = (bolt.life * 4.0).clamp(0.0, 1.0);

            if bolt.brightness < 0.01 {
                continue;
            }

            for &(x1, y1, x2, y2) in &bolt.segments {
                // Bresenham-like line drawing
                let steps = ((x2 - x1).abs().max((y2 - y1).abs()) as usize).max(1);
                for i in 0..=steps {
                    let t = i as f64 / steps as f64;
                    let px = (x1 + (x2 - x1) * t) as usize;
                    let py = (y1 + (y2 - y1) * t) as usize;

                    // Draw bolt with glow
                    for gy in -1i32..=1 {
                        for gx in -1i32..=1 {
                            let gx_pos = (px as i32 + gx) as usize;
                            let gy_pos = (py as i32 + gy) as usize;
                            if gx_pos < canvas.width && gy_pos < canvas.height {
                                let dist = ((gx * gx + gy * gy) as f64).sqrt();
                                let glow = bolt.brightness * (1.0 - dist * 0.4).max(0.0);
                                let b_val = (200.0 + 55.0 * glow) as u8;
                                let g_val = (180.0 + 75.0 * glow) as u8;
                                canvas.set_colored(gx_pos, gy_pos, glow, g_val, g_val, b_val);
                            }
                        }
                    }
                }
            }
        }

        // Remove dead bolts
        self.bolts.retain(|b| b.life > 0.0);
    }
}
