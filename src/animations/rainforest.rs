use super::Animation;
use crate::render::{Canvas, RenderMode};
use rand::RngExt;

// --- Color constants ---

const BG_MOUNTAIN_DARK: (u8, u8, u8) = (10, 30, 25);
const BG_MOUNTAIN_LIGHT: (u8, u8, u8) = (20, 50, 40);
const MID_TRUNK: (u8, u8, u8) = (40, 80, 30);
const MID_CANOPY_DARK: (u8, u8, u8) = (15, 90, 25);
const MID_CANOPY_LIGHT: (u8, u8, u8) = (30, 140, 40);
const FG_TRUNK: (u8, u8, u8) = (100, 60, 25);
const FG_TRUNK_DARK: (u8, u8, u8) = (70, 40, 15);
const FG_LEAF_DARK: (u8, u8, u8) = (20, 120, 30);
const FG_LEAF_LIGHT: (u8, u8, u8) = (50, 180, 50);
const FERN_COLOR: (u8, u8, u8) = (30, 150, 40);
const LEAF_YELLOW: (u8, u8, u8) = (220, 200, 40);
const LEAF_ORANGE: (u8, u8, u8) = (220, 140, 30);
const LEAF_BROWN: (u8, u8, u8) = (160, 100, 40);
const RAIN_COLOR: (u8, u8, u8) = (140, 180, 220);
const SKY_COLOR: (u8, u8, u8) = (5, 15, 12);

// --- Structs ---

struct MountainPeak {
    x: f64,     // center x position in world space
    width: f64, // half-width of the peak
    height: f64,
}

struct MidTree {
    x: f64,       // trunk base x in world space
    trunk_h: f64, // trunk height in pixels
    canopy_w: f64,
    shade: f64, // 0..1 color variation
}

struct FgTree {
    x: f64, // trunk base x in world space
    trunk_h: f64,
    shade: f64,
    has_vine: bool,
    vine_len: f64,
}

struct Fern {
    x: f64,
    y: f64, // world y (near bottom)
    size: f64,
    facing_right: bool,
}

struct FallingLeaf {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    sway_phase: f64,
    sway_amp: f64,
    color_idx: usize, // 0=yellow, 1=orange, 2=brown
    char_idx: usize,  // index into LEAF_CHARS
}

struct RainDrop {
    x: f64,
    y: f64,
    speed: f64,
    length: f64,
}

struct Bird {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    color_idx: usize,
    wing_phase: f64,
}

const LEAF_CHARS: [char; 3] = ['*', '.', ','];
const LEAF_COLORS: [(u8, u8, u8); 3] = [LEAF_YELLOW, LEAF_ORANGE, LEAF_BROWN];

const BIRD_COLORS: [(u8, u8, u8); 4] = [
    (220, 50, 50),  // red
    (50, 120, 220), // blue
    (240, 220, 40), // yellow
    (50, 200, 150), // teal
];

/// Layered rainforest scene with parallax scrolling, rain, birds, and falling leaves.
pub struct Rainforest {
    width: usize,
    height: usize,
    scale: f64,

    // Scene elements
    mountains: Vec<MountainPeak>,
    mid_trees: Vec<MidTree>,
    fg_trees: Vec<FgTree>,
    ferns: Vec<Fern>,

    // Particles
    leaves: Vec<FallingLeaf>,
    rain_drops: Vec<RainDrop>,
    birds: Vec<Bird>,

    // Weather state
    is_raining: bool,
    rain_timer: f64,     // time until next weather toggle
    rain_intensity: f64, // 0..1 ramp

    // Scroll offsets (world space)
    bg_offset: f64,
    mid_offset: f64,
    fg_offset: f64,

    rng: rand::rngs::ThreadRng,
}

impl Rainforest {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut s = Rainforest {
            width,
            height,
            scale,
            mountains: Vec::new(),
            mid_trees: Vec::new(),
            fg_trees: Vec::new(),
            ferns: Vec::new(),
            leaves: Vec::new(),
            rain_drops: Vec::new(),
            birds: Vec::new(),
            is_raining: false,
            rain_timer: 8.0,
            rain_intensity: 0.0,
            bg_offset: 0.0,
            mid_offset: 0.0,
            fg_offset: 0.0,
            rng: rand::rng(),
        };
        s.build_scene();
        s
    }

    fn build_scene(&mut self) {
        let w = self.width as f64;
        let h = self.height as f64;
        let rng = &mut self.rng;

        // Background mountains: 4-6 peaks spanning 3x the screen width for wrap-around
        let world_w = w * 3.0;
        let num_peaks = rng.random_range(4usize..=6);
        self.mountains = (0..num_peaks)
            .map(|i| MountainPeak {
                x: world_w * (i as f64 + 0.5) / num_peaks as f64,
                width: rng.random_range(w * 0.3..w * 0.7),
                height: rng.random_range(h * 0.2..h * 0.45),
            })
            .collect();

        // Mid-ground trees: spaced across 3x screen width
        let num_mid = ((w / 12.0) * self.scale).clamp(5.0, 30.0) as usize;
        self.mid_trees = (0..num_mid)
            .map(|_| MidTree {
                x: rng.random_range(0.0..world_w),
                trunk_h: rng.random_range(3.0..6.0),
                canopy_w: rng.random_range(3.0..7.0),
                shade: rng.random_range(0.0..1.0),
            })
            .collect();

        // Foreground trees: fewer, larger, across 3x screen width
        let num_fg = ((w / 20.0) * self.scale).clamp(2.0, 10.0) as usize;
        self.fg_trees = (0..num_fg)
            .map(|_| FgTree {
                x: rng.random_range(0.0..world_w),
                trunk_h: rng.random_range(h * 0.3..h * 0.7),
                shade: rng.random_range(0.0..1.0),
                has_vine: rng.random_bool(0.5),
                vine_len: rng.random_range(3.0..8.0),
            })
            .collect();

        // Ferns along the ground
        let num_ferns = ((w / 8.0) * self.scale).clamp(3.0, 15.0) as usize;
        self.ferns = (0..num_ferns)
            .map(|_| Fern {
                x: rng.random_range(0.0..world_w),
                y: h - 2.0,
                size: rng.random_range(2.0..5.0),
                facing_right: rng.random_bool(0.5),
            })
            .collect();

        // Falling leaves
        let num_leaves = ((w * h / 400.0) * self.scale).clamp(5.0, 40.0) as usize;
        self.leaves = (0..num_leaves)
            .map(|_| FallingLeaf {
                x: rng.random_range(0.0..w),
                y: rng.random_range(-(h * 0.5)..h),
                vx: rng.random_range(-1.0..1.0),
                vy: rng.random_range(1.5..4.0),
                sway_phase: rng.random_range(0.0..std::f64::consts::TAU),
                sway_amp: rng.random_range(0.5..2.0),
                color_idx: rng.random_range(0..3),
                char_idx: rng.random_range(0..3),
            })
            .collect();

        // Rain starts empty, populated when weather toggles
        self.rain_drops.clear();

        // Birds start empty, spawned randomly during update
        self.birds.clear();
    }
}

impl Animation for Rainforest {
    fn name(&self) -> &str {
        "rainforest"
    }

    fn preferred_render(&self) -> RenderMode {
        RenderMode::Ascii
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.build_scene();
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let w = self.width;
        let h = self.height;
        let fw = w as f64;
        let fh = h as f64;
        let world_w = fw * 3.0;

        // --- Scroll offsets (parallax speeds) ---
        self.bg_offset = (time * 2.0) % world_w;
        self.mid_offset = (time * 5.0) % world_w;
        self.fg_offset = (time * 10.0) % world_w;

        // --- Weather toggle ---
        self.rain_timer -= dt;
        if self.rain_timer <= 0.0 {
            self.is_raining = !self.is_raining;
            self.rain_timer = if self.is_raining {
                self.rng.random_range(6.0..12.0)
            } else {
                self.rng.random_range(8.0..15.0)
            };
            if self.is_raining {
                // Populate rain drops
                let num_drops = ((fw * fh / 60.0) * self.scale).clamp(10.0, 80.0) as usize;
                self.rain_drops = (0..num_drops)
                    .map(|_| RainDrop {
                        x: self.rng.random_range(0.0..fw),
                        y: self.rng.random_range(-(fh * 0.5)..fh),
                        speed: self.rng.random_range(20.0..45.0),
                        length: self.rng.random_range(1.0..3.0),
                    })
                    .collect();
            } else {
                self.rain_drops.clear();
            }
        }

        // Ramp rain intensity
        let target = if self.is_raining { 1.0 } else { 0.0 };
        self.rain_intensity += (target - self.rain_intensity) * dt * 2.0;

        canvas.clear();

        // === 1. SKY BACKGROUND ===
        for y in 0..h {
            for x in 0..w {
                // Dark gradient from top to bottom
                let t = y as f64 / fh;
                let r = (SKY_COLOR.0 as f64 * (1.0 - t * 0.5)) as u8;
                let g = (SKY_COLOR.1 as f64 * (1.0 - t * 0.3)) as u8;
                let b = (SKY_COLOR.2 as f64 * (1.0 - t * 0.2)) as u8;
                canvas.set_colored(x, y, 0.15, r, g, b);
            }
        }

        // === 2. BACKGROUND MOUNTAINS (silhouettes, slowest scroll) ===
        for peak in &self.mountains {
            let cx = ((peak.x - self.bg_offset) % world_w + world_w) % world_w;
            // Draw the peak if it's visible (within screen + margin)
            let _left = cx - peak.width;
            let _right = cx + peak.width;
            // We may need to draw wrapped copies
            for copy_offset in &[-world_w, 0.0, world_w] {
                let draw_cx = cx + copy_offset;
                let draw_left = draw_cx - peak.width;
                let draw_right = draw_cx + peak.width;
                if draw_right < 0.0 || draw_left >= fw {
                    continue;
                }
                let x_start = draw_left.max(0.0) as usize;
                let x_end = draw_right.min(fw) as usize;
                for px in x_start..x_end {
                    let dist = (px as f64 - draw_cx).abs() / peak.width;
                    if dist < 1.0 {
                        let peak_h = peak.height * (1.0 - dist * dist); // parabolic profile
                        let base_y = fh * 0.6; // mountains sit in lower half of sky
                        let top_y = (base_y - peak_h).max(0.0) as usize;
                        let bot_y = base_y as usize;
                        for py in top_y..bot_y.min(h) {
                            let depth_t = (py - top_y) as f64 / (bot_y - top_y).max(1) as f64;
                            let r = (BG_MOUNTAIN_DARK.0 as f64
                                + (BG_MOUNTAIN_LIGHT.0 - BG_MOUNTAIN_DARK.0) as f64
                                    * (1.0 - depth_t)) as u8;
                            let g = (BG_MOUNTAIN_DARK.1 as f64
                                + (BG_MOUNTAIN_LIGHT.1 - BG_MOUNTAIN_DARK.1) as f64
                                    * (1.0 - depth_t)) as u8;
                            let b = (BG_MOUNTAIN_DARK.2 as f64
                                + (BG_MOUNTAIN_LIGHT.2 - BG_MOUNTAIN_DARK.2) as f64
                                    * (1.0 - depth_t)) as u8;
                            canvas.set_colored(px, py, 0.4 + 0.2 * (1.0 - depth_t), r, g, b);
                        }
                    }
                }
            }
        }

        // === 3. MID-GROUND TREES (medium scroll speed) ===
        for tree in &self.mid_trees {
            let tx = ((tree.x - self.mid_offset) % world_w + world_w) % world_w;
            for copy_offset in &[-world_w, 0.0, world_w] {
                let draw_x = tx + copy_offset;
                let ix = draw_x as isize;
                // Trunk
                let trunk_base = fh * 0.75;
                let trunk_top = trunk_base - tree.trunk_h;
                let trunk_char = '|';
                let (tr_r, tr_g, tr_b) = lerp_color(MID_TRUNK, MID_CANOPY_DARK, 0.3);
                if ix >= 0 && ix < w as isize {
                    let ty_start = trunk_top.max(0.0) as usize;
                    let ty_end = trunk_base.min(fh) as usize;
                    for ty in ty_start..ty_end {
                        canvas.set_char(ix as usize, ty, trunk_char, tr_r, tr_g, tr_b);
                    }
                }
                // Canopy: draw rows of W and \ | / above trunk
                let canopy_top = (trunk_top - tree.canopy_w * 0.6).max(0.0);
                let canopy_rows = tree.canopy_w as usize;
                let shade = tree.shade;
                for (row_i, _row_y) in (0..canopy_rows).enumerate() {
                    let cy = (canopy_top as usize + row_i).min(h - 1);
                    let half_w = if row_i < canopy_rows / 2 {
                        row_i + 1
                    } else {
                        canopy_rows - row_i
                    };
                    let cx = ix;
                    let (cr, cg, cb) = lerp_color(MID_CANOPY_DARK, MID_CANOPY_LIGHT, shade);
                    for dx in -(half_w as isize)..=(half_w as isize) {
                        let px = cx + dx;
                        if px >= 0 && px < w as isize && cy < h {
                            let ch = if dx == 0 && row_i < canopy_rows - 1 {
                                '|'
                            } else if dx < 0 {
                                '\\'
                            } else if dx > 0 {
                                '/'
                            } else {
                                'W'
                            };
                            canvas.set_char(px as usize, cy, ch, cr, cg, cb);
                        }
                    }
                }
            }
        }

        // === 4. FOREGROUND TREES (fastest scroll, largest) ===
        for tree in &self.fg_trees {
            let tx = ((tree.x - self.fg_offset) % world_w + world_w) % world_w;
            for copy_offset in &[-world_w, 0.0, world_w] {
                let draw_x = tx + copy_offset;
                let ix = draw_x as isize;
                if ix < -5 || ix > w as isize + 5 {
                    continue;
                }

                let trunk_base = fh - 1.0;
                let trunk_top = (trunk_base - tree.trunk_h).max(0.0);
                let shade = tree.shade;

                // Trunk: thick (2 chars wide) brown
                let (tk_r, tk_g, tk_b) = lerp_color(FG_TRUNK_DARK, FG_TRUNK, shade);
                let ty_start = trunk_top as usize;
                let ty_end = trunk_base.min(fh) as usize;
                for ty in ty_start..ty_end {
                    for dx in 0..2 {
                        let px = ix + dx;
                        if px >= 0 && px < w as isize {
                            canvas.set_char(px as usize, ty, '|', tk_r, tk_g, tk_b);
                        }
                    }
                }

                // Canopy at top of trunk: cluster of Y, *, ~ characters
                let canopy_h = (tree.trunk_h * 0.35).min(6.0) as usize;
                let (cl_r, cl_g, cl_b) = lerp_color(FG_LEAF_DARK, FG_LEAF_LIGHT, shade);
                for dy in 0..canopy_h {
                    let cy = trunk_top as isize - dy as isize - 1;
                    if cy < 0 {
                        break;
                    }
                    let spread = (dy + 1) as isize;
                    for dx in -(spread)..=(spread) {
                        let px = ix + dx;
                        if px >= 0 && px < w as isize && cy < h as isize {
                            let ch = if dx == 0 {
                                'Y'
                            } else if dx.abs() == spread {
                                '~'
                            } else {
                                '*'
                            };
                            canvas.set_char(px as usize, cy as usize, ch, cl_r, cl_g, cl_b);
                        }
                    }
                }

                // Vine hanging from trunk
                if tree.has_vine && ix >= 0 && ix + 2 < w as isize {
                    let vine_x = ix + 2;
                    let vine_start = trunk_top + tree.trunk_h * 0.2;
                    let vine_end = vine_start + tree.vine_len;
                    let vy_start = vine_start.max(0.0) as usize;
                    let vy_end = vine_end.min(fh) as usize;
                    for vy in vy_start..vy_end {
                        let sway = ((time * 2.0 + vy as f64 * 0.3).sin() * 0.8) as isize;
                        let px = vine_x + sway;
                        if px >= 0 && px < w as isize {
                            canvas.set_char(px as usize, vy, '~', 30, 130, 40);
                        }
                    }
                }
            }
        }

        // === 5. FERNS along the ground ===
        for fern in &self.ferns {
            let fx = ((fern.x - self.fg_offset) % world_w + world_w) % world_w;
            for copy_offset in &[-world_w, 0.0, world_w] {
                let draw_x = fx + copy_offset;
                let ix = draw_x as isize;
                if ix < -6 || ix > w as isize + 6 {
                    continue;
                }
                let base_y = fern.y;
                let size = fern.size as isize;
                let dir: isize = if fern.facing_right { 1 } else { -1 };

                // Stem
                let stem_chars: &[char] = &['/', '|', '\\'];
                for (i, &ch) in stem_chars.iter().enumerate() {
                    let py = (base_y - i as f64) as isize;
                    let px = ix + dir * i as isize;
                    if px >= 0 && px < w as isize && py >= 0 && py < h as isize {
                        canvas.set_char(
                            px as usize,
                            py as usize,
                            ch,
                            FERN_COLOR.0,
                            FERN_COLOR.1,
                            FERN_COLOR.2,
                        );
                    }
                }
                // Fronds extending from stem
                for i in 1..=size {
                    let frond_y = (base_y - (i as f64 + 1.0)) as isize;
                    if frond_y < 0 {
                        break;
                    }
                    let frond_len = (size - i + 1).min(3);
                    for d in 1..=frond_len {
                        let px1 = ix + dir * i + dir * d;
                        let px2 = ix + dir * i - dir * d;
                        if px1 >= 0 && px1 < w as isize {
                            canvas.set_char(
                                px1 as usize,
                                frond_y as usize,
                                '~',
                                FERN_COLOR.0,
                                FERN_COLOR.1,
                                FERN_COLOR.2,
                            );
                        }
                        if px2 >= 0 && px2 < w as isize && d > 0 {
                            canvas.set_char(
                                px2 as usize,
                                frond_y as usize,
                                '~',
                                FERN_COLOR.0,
                                FERN_COLOR.1,
                                FERN_COLOR.2,
                            );
                        }
                    }
                }
            }
        }

        // === 6. FALLING LEAVES ===
        for leaf in &mut self.leaves {
            leaf.sway_phase += dt * 2.5;
            let sway = (leaf.sway_phase).sin() * leaf.sway_amp;
            leaf.x += (leaf.vx + sway) * dt;
            leaf.y += leaf.vy * dt;

            // Reset when off-screen
            if leaf.y > fh + 5.0 || leaf.x < -10.0 || leaf.x > fw + 10.0 {
                leaf.x = self.rng.random_range(0.0..fw);
                leaf.y = self.rng.random_range(-15.0..-2.0);
                leaf.vy = self.rng.random_range(1.5..4.0);
                leaf.vx = self.rng.random_range(-1.0..1.0);
                leaf.color_idx = self.rng.random_range(0..3);
                leaf.char_idx = self.rng.random_range(0..3);
            }

            let px = leaf.x as usize;
            let py = leaf.y as usize;
            if px < w && py < h {
                let c = LEAF_COLORS[leaf.color_idx];
                canvas.set_char(px, py, LEAF_CHARS[leaf.char_idx], c.0, c.1, c.2);
            }
        }

        // === 7. RAIN (when active) ===
        if self.rain_intensity > 0.01 {
            let wind = (time * 0.4).sin() * 3.0;
            for drop in &mut self.rain_drops {
                drop.x += wind * dt;
                drop.y += drop.speed * dt;

                if drop.y > fh + 5.0 {
                    drop.y = self.rng.random_range(-10.0..0.0);
                    drop.x = self.rng.random_range(0.0..fw);
                    drop.speed = self.rng.random_range(20.0..45.0);
                }
                if drop.x < 0.0 {
                    drop.x += fw;
                } else if drop.x >= fw {
                    drop.x -= fw;
                }

                // Draw rain drop as angled character
                let px = drop.x as usize;
                let py = drop.y as usize;
                if px < w && py < h {
                    let ch = if wind > 1.0 {
                        '/'
                    } else if wind < -1.0 {
                        '\\'
                    } else {
                        '|'
                    };
                    let alpha = self.rain_intensity;
                    let r = (RAIN_COLOR.0 as f64 * alpha) as u8;
                    let g = (RAIN_COLOR.1 as f64 * alpha) as u8;
                    let b = (RAIN_COLOR.2 as f64 * alpha) as u8;
                    canvas.set_char(px, py, ch, r, g, b);
                }

                // Draw tail
                let tail_px = (drop.x - wind * 0.05) as usize;
                let tail_py = (drop.y - drop.length) as usize;
                if tail_px < w && tail_py < h {
                    let alpha = self.rain_intensity * 0.5;
                    let r = (RAIN_COLOR.0 as f64 * alpha) as u8;
                    let g = (RAIN_COLOR.1 as f64 * alpha) as u8;
                    let b = (RAIN_COLOR.2 as f64 * alpha) as u8;
                    canvas.set_char(tail_px, tail_py, '.', r, g, b);
                }
            }
        }

        // === 8. BIRDS ===
        // Spawn birds occasionally
        if self.rng.random_range(0.0..1.0) < dt * 0.15 {
            let dir: f64 = if self.rng.random_bool(0.5) { 1.0 } else { -1.0 };
            self.birds.push(Bird {
                x: if dir > 0.0 { -3.0 } else { fw + 3.0 },
                y: self.rng.random_range(1.0..fh * 0.4),
                vx: dir * self.rng.random_range(8.0..18.0),
                vy: self.rng.random_range(-1.0..1.0),
                color_idx: self.rng.random_range(0..4),
                wing_phase: self.rng.random_range(0.0..std::f64::consts::TAU),
            });
        }

        // Update and draw birds
        self.birds.retain(|b| b.x > -10.0 && b.x < fw + 10.0);
        for bird in &mut self.birds {
            bird.x += bird.vx * dt;
            bird.y += bird.vy * dt + (time * 3.0 + bird.wing_phase).sin() * 0.3 * dt;
            bird.wing_phase += dt * 8.0;

            let px = bird.x as usize;
            let py = bird.y as usize;
            if px < w && py < h {
                let wing_up = bird.wing_phase.sin() > 0.0;
                let ch = if bird.vx > 0.0 {
                    if wing_up { '>' } else { 'v' }
                } else if wing_up {
                    '<'
                } else {
                    'v'
                };
                let c = BIRD_COLORS[bird.color_idx];
                canvas.set_char(px, py, ch, c.0, c.1, c.2);
            }
        }
    }
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t) as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t) as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * t) as u8,
    )
}
