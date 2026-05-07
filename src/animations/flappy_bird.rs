use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// A single pipe obstacle with a gap for the bird to fly through
struct Pipe {
    x: f64,
    gap_center: f64,
    scored: bool,
}

/// Self-playing Flappy Bird with AI controller
pub struct FlappyBird {
    width: usize,
    height: usize,

    // Bird state
    bird_y: f64,
    bird_vy: f64,
    bird_x: f64,

    // Game state
    pipes: Vec<Pipe>,
    score: u32,
    game_over_timer: f64,
    pipe_timer: f64,

    // Tuning (scaled from height)
    gravity: f64,
    flap_strength: f64,
    pipe_speed: f64,
    pipe_spacing: f64,
    gap_size: f64,

    rng: rand::rngs::ThreadRng,
}

impl FlappyBird {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let mut fb = FlappyBird {
            width,
            height,
            bird_y: height as f64 * 0.5,
            bird_vy: 0.0,
            bird_x: width as f64 * 0.2,
            pipes: Vec::new(),
            score: 0,
            game_over_timer: 0.0,
            pipe_timer: 0.0,
            gravity: 0.0,
            flap_strength: 0.0,
            pipe_speed: 0.0,
            pipe_spacing: 0.0,
            gap_size: 0.0,
            rng: rand::rng(),
        };
        fb.tune_params();
        fb
    }

    /// Recalculate tuning parameters based on canvas dimensions
    fn tune_params(&mut self) {
        let h = self.height as f64;
        let w = self.width as f64;

        self.gravity = h * 1.8;
        self.flap_strength = h * 0.55;
        self.pipe_speed = w * 0.35;
        self.gap_size = (h * 0.3).max(6.0);
        self.pipe_spacing = (w * 0.35).max(15.0);
        self.bird_x = w * 0.2;
    }

    fn reset(&mut self) {
        let h = self.height as f64;
        self.bird_y = h * 0.5;
        self.bird_vy = 0.0;
        self.pipes.clear();
        self.score = 0;
        self.pipe_timer = 0.0;
        // Spawn first pipe ahead of the bird
        self.spawn_pipe(self.width as f64 * 0.7);
    }

    fn spawn_pipe(&mut self, x: f64) {
        let h = self.height as f64;
        let half_gap = self.gap_size * 0.5;
        let margin = half_gap + 2.0;
        let gap_center = self.rng.random_range(margin..h - margin);
        self.pipes.push(Pipe {
            x,
            gap_center,
            scored: false,
        });
    }

    /// AI controller: flap when the bird is below the next pipe's gap center
    /// and still moving downward. Simple but effective.
    fn ai_should_flap(&self) -> bool {
        let next_pipe = self
            .pipes
            .iter()
            .filter(|p| p.x + 2.0 > self.bird_x)
            .min_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

        let target_y = match next_pipe {
            Some(p) => p.gap_center,
            None => self.height as f64 * 0.5,
        };

        // Flap when below target and falling (or barely rising)
        self.bird_y > target_y && self.bird_vy >= 0.0
    }
}

impl Animation for FlappyBird {
    fn name(&self) -> &str {
        "flappy_bird"
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.tune_params();
        self.reset();
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = self.width as f64;
        let h = self.height as f64;

        // Handle game over pause
        if self.game_over_timer > 0.0 {
            self.game_over_timer -= dt;
            if self.game_over_timer <= 0.0 {
                self.reset();
            }
        } else {
            // AI decision
            if self.ai_should_flap() {
                self.bird_vy = -self.flap_strength;
            }

            // Physics
            self.bird_vy += self.gravity * dt;
            self.bird_y += self.bird_vy * dt;

            // Pipe spawning
            self.pipe_timer += dt;
            if self.pipe_timer >= self.pipe_spacing / self.pipe_speed {
                self.pipe_timer = 0.0;
                self.spawn_pipe(w + 3.0);
            }

            // Move pipes
            for pipe in &mut self.pipes {
                pipe.x -= self.pipe_speed * dt;
            }

            // Score: check if bird passed a pipe
            for pipe in &mut self.pipes {
                if !pipe.scored && pipe.x + 2.0 < self.bird_x {
                    pipe.scored = true;
                    self.score += 1;
                }
            }

            // Remove off-screen pipes
            self.pipes.retain(|p| p.x > -5.0);

            // Collision detection
            let bird_radius = 1.0;

            // Floor / ceiling
            if self.bird_y < bird_radius || self.bird_y >= h - bird_radius {
                self.game_over_timer = 2.0;
            }

            // Pipes
            let pipe_width = 3.0;
            for pipe in &self.pipes {
                let pipe_left = pipe.x;
                let pipe_right = pipe.x + pipe_width;

                if self.bird_x + bird_radius > pipe_left && self.bird_x - bird_radius < pipe_right {
                    let half_gap = self.gap_size * 0.5;
                    let gap_top = pipe.gap_center - half_gap;
                    let gap_bot = pipe.gap_center + half_gap;

                    if self.bird_y - bird_radius < gap_top || self.bird_y + bird_radius > gap_bot {
                        self.game_over_timer = 2.0;
                        break;
                    }
                }
            }
        }

        // === Render ===
        canvas.clear();

        // Sky gradient: darker blue at top, lighter at bottom
        for y in 0..self.height {
            let frac = y as f64 / h;
            // Top: deep sky blue (60, 120, 200), bottom: light blue (140, 200, 240)
            let r = (60.0 + 80.0 * frac) as u8;
            let g = (120.0 + 80.0 * frac) as u8;
            let b = (200.0 + 40.0 * frac) as u8;
            for x in 0..self.width {
                canvas.set_colored(x, y, 0.25, r, g, b);
            }
        }

        // Ground strip at the bottom
        let ground_y = self.height.saturating_sub(2);
        for y in ground_y..self.height {
            for x in 0..self.width {
                canvas.set_colored(x, y, 0.5, 139, 119, 42);
            }
        }

        let pipe_width = 3.0;
        let half_gap = self.gap_size * 0.5;

        // Draw pipes
        for pipe in &self.pipes {
            let px_left = pipe.x as usize;
            let pipe_w = pipe_width as usize;
            let gap_top = (pipe.gap_center - half_gap) as usize;
            let gap_bot = (pipe.gap_center + half_gap) as usize;

            // Top pipe (from y=0 down to gap_top)
            for dx in 0..pipe_w {
                let x = px_left + dx;
                if x >= self.width {
                    continue;
                }
                for y in 0..gap_top.min(ground_y) {
                    // Pipe body: darker green edge, lighter center
                    let is_edge = dx == 0 || dx == pipe_w - 1;
                    let brightness = if is_edge { 0.6 } else { 0.85 };
                    canvas.set_colored(x, y, brightness, 34, 139, 34);
                }

                // Pipe cap at the bottom of the top pipe
                if gap_top > 0 && gap_top <= ground_y {
                    let cap_y = gap_top.saturating_sub(1);
                    if cap_y < self.height {
                        canvas.set_colored(x, cap_y, 0.95, 50, 180, 50);
                    }
                }
            }

            // Bottom pipe (from gap_bot down to ground)
            for dx in 0..pipe_w {
                let x = px_left + dx;
                if x >= self.width {
                    continue;
                }
                for y in gap_bot.min(ground_y)..ground_y {
                    let is_edge = dx == 0 || dx == pipe_w - 1;
                    let brightness = if is_edge { 0.6 } else { 0.85 };
                    canvas.set_colored(x, y, brightness, 34, 139, 34);
                }

                // Pipe cap at the top of the bottom pipe
                if gap_bot < ground_y {
                    let cap_y = gap_bot;
                    if cap_y < self.height {
                        canvas.set_colored(x, cap_y, 0.95, 50, 180, 50);
                    }
                }
            }
        }

        // Draw bird (yellow blob, 3x2 area)
        let bx = self.bird_x as usize;
        let by = self.bird_y as usize;
        let is_dead = self.game_over_timer > 0.0;

        // Bird body
        for dy in 0..2_usize {
            for dx in 0..3_usize {
                let px = bx + dx;
                let py = by + dy;
                if px < self.width && py < self.height && py < ground_y {
                    if is_dead {
                        canvas.set_colored(px, py, 0.9, 220, 60, 60);
                    } else {
                        canvas.set_colored(px, py, 1.0, 255, 220, 40);
                    }
                }
            }
        }

        // Bird eye (white pixel) and beak
        if bx + 2 < self.width && by < self.height && by < ground_y {
            canvas.set_colored(bx + 2, by, 1.0, 255, 255, 255);
        }
        if bx + 3 < self.width && by + 1 < self.height && by + 1 < ground_y {
            canvas.set_colored(bx + 3, by + 1, 1.0, 255, 160, 40);
        }

        // Bird wing highlight
        if bx + 1 < self.width && by + 1 < self.height && by + 1 < ground_y {
            canvas.set_colored(bx + 1, by + 1, 0.9, 255, 200, 20);
        }

        // Score display using set_char in top-left area
        let score_str = format!("{}", self.score);
        let score_chars: Vec<char> = score_str.chars().collect();
        for (i, ch) in score_chars.iter().enumerate() {
            let sx = 2 + i * 4;
            if sx + 2 < self.width && 1 < self.height {
                // Shadow
                canvas.set_char(sx + 1, 1, *ch, 0, 0, 0);
                // Score digit
                canvas.set_char(sx, 0, *ch, 255, 255, 255);
            }
        }

        // "GAME OVER" flash during reset pause
        if self.game_over_timer > 0.0 {
            let msg = "GAME OVER";
            let msg_start = (self.width / 2).saturating_sub(msg.len() / 2);
            let msg_y = self.height / 2;
            let flash = (self.game_over_timer * 4.0).sin().abs() > 0.3;
            if flash {
                for (i, ch) in msg.chars().enumerate() {
                    let x = msg_start + i;
                    if x < self.width && msg_y < self.height {
                        canvas.set_char(x, msg_y, ch, 255, 80, 80);
                    }
                }
            }
        }
    }
}
