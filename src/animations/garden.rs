use super::Animation;
use crate::render::{Canvas, RenderMode};
use rand::RngExt;

/// A slice of (column_offset, character, is_colored) tuples describing one row of a plant shape.
/// The `'static` lifetime means these are compile-time constant arrays embedded in the binary.
type PRow = &'static [(i32, char, bool)];

// Variety 0: Rose — orange bloom
const V0: &[PRow] = &[
    &[(0, '~', false)],
    &[(0, '|', false)],
    &[(0, 'Y', false)],
    &[(0, 'Y', false)],
    &[(-1, '(', true), (0, '@', true), (1, ')', true)],
];

// Variety 1: Daisy — blue bloom
const V1: &[PRow] = &[
    &[(-1, ',', false), (0, ',', false)],
    &[(0, 'J', false)],
    &[(0, '|', false)],
    &[(0, '*', true)],
    &[(-1, 'o', true), (0, '*', true), (1, 'o', true)],
];

// Variety 2: Tulip — magenta bloom
const V2: &[PRow] = &[
    &[(0, ',', false)],
    &[(0, 'b', false)],
    &[(0, '|', false)],
    &[(-1, '(', true), (1, ')', true)],
    &[(-1, '{', true), (0, 'a', true), (1, '}', true)],
];

// Variety 3: Tree — bright green canopy
const V3: &[PRow] = &[
    &[(-1, '~', false), (0, '~', false)],
    &[(0, '|', false)],
    &[(0, 'Y', false)],
    &[(-1, '\\', false), (0, '|', false), (1, '/', false)],
    &[(-1, 'W', true), (0, 'W', true), (1, 'W', true)],
];

// Variety 4: Sunflower — yellow bloom
const V4: &[PRow] = &[
    &[(0, 'Y', false)],
    &[(0, '|', false)],
    &[(0, '|', false)],
    &[(-1, '(', true), (0, '|', false), (1, ')', true)],
    &[(-1, '*', true), (0, '@', true), (1, '*', true)],
];

// Variety 5: Fantasy — purple cap
const V5: &[PRow] = &[
    &[(-1, ',', false), (0, ',', false)],
    &[(0, '|', false)],
    &[(-1, '|', false), (0, '|', false), (1, '|', false)],
    &[(0, '|', false)],
    &[(-1, '(', true), (0, 'o', true), (1, ')', true)],
];

const VARIETIES: &[&[PRow]] = &[V0, V1, V2, V3, V4, V5];

const FLOWER_COLORS: [(u8, u8, u8); 6] = [
    (255, 100, 30),  // orange  — rose
    (100, 180, 255), // blue    — daisy
    (220, 80, 200),  // magenta — tulip
    (80, 220, 80),   // green   — tree
    (255, 220, 0),   // yellow  — sunflower
    (180, 100, 255), // purple  — fantasy
];

const STEM_COLOR: (u8, u8, u8) = (60, 180, 60);
const GROUND_COLOR: (u8, u8, u8) = (120, 80, 40);
const SUN_COLOR: (u8, u8, u8) = (255, 220, 50);
const CLOUD_COLOR: (u8, u8, u8) = (200, 200, 220);
const RAIN_COLOR: (u8, u8, u8) = (150, 200, 255);
const SKY_DIM: (u8, u8, u8) = (15, 25, 50);

// Static rows used to build rose shapes dynamically at spawn time
static ROSE_STEM: &[(i32, char, bool)] = &[(0, '|', false)];
static ROSE_STEM_LEAF: &[(i32, char, bool)] = &[(0, '|', false), (1, '~', false)];
static ROSE_BRANCH: &[(i32, char, bool)] = &[(0, 'Y', false)];
static ROSE_BLOOM: &[(i32, char, bool)] = &[(-1, '(', true), (0, '@', true), (1, ')', true)];

struct Plant {
    col: usize,
    variety: usize,
    stage: usize,     // 0 = seed; shape.len() = full bloom
    shape: Vec<PRow>, // rows bottom→top, generated at spawn
}

struct Raindrop {
    x: f64,
    y: f64,
    speed: f64,
}

struct Cloud {
    x: f64,
    width: usize,
    raining: bool,
    rain_timer: f64,    // seconds left in current rain burst
    rain_cooldown: f64, // seconds until next rain burst
    spawn_timer: f64,   // seconds until next raindrop spawns
}

struct Splash {
    x: usize,
    y: usize,
    ttl: f64,
}

/// Growing garden with sun, drifting clouds, rain, and six blooming plant varieties.
pub struct Garden {
    plants: Vec<Plant>,
    clouds: Vec<Cloud>,
    drops: Vec<Raindrop>,
    splashes: Vec<Splash>,
    width: usize,
    height: usize,
    rng: rand::rngs::ThreadRng,
}

impl Garden {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();

        let num_plants = ((width as f64 / 8.0) * scale).clamp(3.0, 20.0) as usize;
        let spacing = (width / (num_plants + 1)).max(1);
        let plants: Vec<Plant> = (0..num_plants)
            .map(|i| {
                let variety = rng.random_range(0..VARIETIES.len());
                let shape: Vec<PRow> = if variety == 0 {
                    // Rose: plain stems × 1-3, leafed stems × 1-3, branch, bloom
                    // Interleave plain | and leafed |~ rows (2–6 total)
                    // so they're never all bunched together
                    let stem_rows = rng.random_range(2..=6_usize);
                    let mut s: Vec<PRow> = Vec::new();
                    for i in 0..stem_rows {
                        // Alternate base pattern (even=plain, odd=leafed)
                        // with a 30% chance of flipping for natural variation
                        let leafed = (i % 2 == 1) ^ rng.random_bool(0.3);
                        s.push(if leafed { ROSE_STEM_LEAF } else { ROSE_STEM });
                    }
                    s.push(ROSE_BRANCH);
                    s.push(ROSE_BLOOM);
                    s
                } else {
                    // Other varieties: always use the full shape so Y/branch
                    // characters never appear at the bottom during early growth
                    VARIETIES[variety].to_vec()
                };
                Plant {
                    col: spacing * (i + 1),
                    variety,
                    stage: 0,
                    shape,
                }
            })
            .collect();

        let num_clouds = ((width as f64 / 40.0) * scale).clamp(1.0, 4.0) as usize;
        let clouds: Vec<Cloud> = (0..num_clouds)
            .map(|i| Cloud {
                x: (width as f64 / num_clouds as f64) * i as f64,
                width: rng.random_range(6..14),
                raining: false,
                rain_timer: 0.0,
                rain_cooldown: rng.random_range(3.0..12.0),
                spawn_timer: 0.0,
            })
            .collect();

        Garden {
            plants,
            clouds,
            drops: Vec::new(),
            splashes: Vec::new(),
            width,
            height,
            rng: rand::rng(),
        }
    }
}

impl Animation for Garden {
    fn name(&self) -> &str {
        "garden"
    }

    fn preferred_render(&self) -> RenderMode {
        RenderMode::Ascii
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        if self.height < 5 {
            return;
        }

        let ground_y = self.height - 1;
        let cloud_y: usize = 3;

        canvas.clear();

        // Sky background (faint tint so chars on black are visible)
        for y in 0..ground_y {
            for x in 0..self.width {
                canvas.set_colored(x, y, 0.1, SKY_DIM.0, SKY_DIM.1, SKY_DIM.2);
            }
        }

        // Ground row
        for x in 0..self.width {
            canvas.set_char(
                x,
                ground_y,
                '=',
                GROUND_COLOR.0,
                GROUND_COLOR.1,
                GROUND_COLOR.2,
            );
        }

        // Static large starburst sun anchored at top-right corner.
        // Center (@) sits at (width-1, 1); rays pointing right/up clip naturally.
        // (dx, dy, char) relative to center
        let sun_cx = self.width as i32 - 1;
        let sun_cy = 1_i32;
        for &(dx, dy, ch) in &[
            // Body + left horizontal arm
            (-5, 0, '*'),
            (-4, 0, '-'),
            (-3, 0, '-'),
            (-2, 0, '-'),
            (-1, 0, '('),
            (0, 0, '@'),
            // Top vertical (single step — rows above 0 clip)
            (0, -1, '|'),
            // NW diagonal (one step visible at row 0)
            (-1, -1, '\\'),
            // Bottom vertical arm
            (0, 1, '|'),
            (0, 2, '|'),
            (0, 3, '*'),
            // SW diagonal arm
            (-1, 1, '/'),
            (-2, 2, '/'),
            (-3, 3, '/'),
        ] {
            let px = sun_cx + dx;
            let py = sun_cy + dy;
            if px >= 0 && py >= 0 {
                canvas.set_char(
                    px as usize,
                    py as usize,
                    ch,
                    SUN_COLOR.0,
                    SUN_COLOR.1,
                    SUN_COLOR.2,
                );
            }
        }

        // Update clouds, collect new raindrops
        let mut new_drops: Vec<Raindrop> = Vec::new();
        for cloud in &mut self.clouds {
            // Drift
            cloud.x += 5.0 * dt;
            if cloud.x > (self.width + cloud.width + 2) as f64 {
                cloud.x = -(cloud.width as f64 + 2.0);
                cloud.width = self.rng.random_range(6..14);
                cloud.raining = false;
                cloud.rain_cooldown = self.rng.random_range(3.0..12.0);
            }

            // State machine
            if cloud.raining {
                cloud.rain_timer -= dt;
                if cloud.rain_timer <= 0.0 {
                    cloud.raining = false;
                    cloud.rain_cooldown = self.rng.random_range(8.0..20.0);
                } else {
                    cloud.spawn_timer -= dt;
                    if cloud.spawn_timer <= 0.0 {
                        cloud.spawn_timer = 0.2;
                        let drop_x = cloud.x + self.rng.random_range(0.0..cloud.width as f64);
                        if drop_x >= 0.0 && (drop_x as usize) < self.width {
                            new_drops.push(Raindrop {
                                x: drop_x,
                                y: (cloud_y + 1) as f64,
                                speed: self.rng.random_range(20.0..35.0),
                            });
                        }
                    }
                }
            } else {
                cloud.rain_cooldown -= dt;
                if cloud.rain_cooldown <= 0.0 {
                    cloud.raining = true;
                    cloud.rain_timer = self.rng.random_range(3.0..8.0);
                    cloud.spawn_timer = 0.0;
                }
            }

            // Draw cloud: (---) or (~~~) when raining
            let cx = cloud.x as i32;
            for i in 0..cloud.width as i32 {
                let px = cx + i;
                if px < 0 || (px as usize) >= self.width {
                    continue;
                }
                let ch = match i {
                    0 => '(',
                    n if n == cloud.width as i32 - 1 => ')',
                    _ => {
                        if cloud.raining {
                            '~'
                        } else {
                            '-'
                        }
                    }
                };
                canvas.set_char(
                    px as usize,
                    cloud_y,
                    ch,
                    CLOUD_COLOR.0,
                    CLOUD_COLOR.1,
                    CLOUD_COLOR.2,
                );
            }
        }
        self.drops.extend(new_drops);

        // Move raindrops; draw in-flight; collect ground hits
        let mut hits: Vec<usize> = Vec::new();
        self.drops.retain_mut(|drop| {
            drop.y += drop.speed * dt;
            if drop.y >= ground_y as f64 {
                hits.push(drop.x as usize);
                false
            } else {
                let x = drop.x as usize;
                let y = drop.y as usize;
                if y > cloud_y {
                    canvas.set_char(x, y, '|', RAIN_COLOR.0, RAIN_COLOR.1, RAIN_COLOR.2);
                }
                true
            }
        });

        // Process hits: grow plants, spawn splashes
        for hit_x in hits {
            for plant in &mut self.plants {
                let lo = plant.col.saturating_sub(2);
                let hi = plant.col + 2;
                if hit_x >= lo && hit_x <= hi && plant.stage < plant.shape.len() {
                    plant.stage += 1;
                }
            }
            if ground_y >= 1 {
                self.splashes.push(Splash {
                    x: hit_x,
                    y: ground_y - 1,
                    ttl: 0.4,
                });
            }
        }

        // Splashes: three characters centred on impact
        self.splashes.retain_mut(|s| {
            s.ttl -= dt;
            if s.ttl > 0.0 {
                for (i, &ch) in ['.', '\'', '.'].iter().enumerate() {
                    let px = s.x as i32 + i as i32 - 1;
                    if px >= 0 {
                        canvas.set_char(
                            px as usize,
                            s.y,
                            ch,
                            RAIN_COLOR.0,
                            RAIN_COLOR.1,
                            RAIN_COLOR.2,
                        );
                    }
                }
                true
            } else {
                false
            }
        });

        // Draw plants
        for plant in &self.plants {
            let (fr, fg, fb) = FLOWER_COLORS[plant.variety];
            let (sr, sg, sb) = STEM_COLOR;

            if plant.stage == 0 {
                if ground_y >= 1 {
                    canvas.set_char(plant.col, ground_y - 1, '.', sr, sg, sb);
                }
                continue;
            }

            let rows = &plant.shape;
            let rows_to_draw = plant.stage.min(rows.len());

            for (row_idx, row) in rows[..rows_to_draw].iter().enumerate() {
                let y = ground_y as i32 - 1 - row_idx as i32;
                if y < 0 {
                    continue;
                }
                // Top drawn row uses flower color; lower rows use stem color
                let is_top = row_idx + 1 == rows_to_draw;
                for &(dx, ch, is_flower) in *row {
                    let px = plant.col as i32 + dx;
                    if px < 0 {
                        continue;
                    }
                    let (r, g, b) = if is_flower && is_top {
                        (fr, fg, fb)
                    } else {
                        (sr, sg, sb)
                    };
                    canvas.set_char(px as usize, y as usize, ch, r, g, b);
                }
            }
        }
    }
}
