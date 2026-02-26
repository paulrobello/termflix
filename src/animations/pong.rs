use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

/// Self-playing Pong with AI paddles
pub struct Pong {
    ball_x: f64,
    ball_y: f64,
    ball_vx: f64,
    ball_vy: f64,
    left_y: f64,
    right_y: f64,
    paddle_h: f64,
    left_score: u32,
    right_score: u32,
    serve_timer: f64,
    rng: rand::rngs::ThreadRng,
}

impl Pong {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let mut rng = rand::rng();
        let w = width as f64;
        let h = height as f64;
        let dir: f64 = if rng.random_range(0u8..2) == 0 {
            1.0
        } else {
            -1.0
        };
        Pong {
            ball_x: w * 0.5,
            ball_y: h * 0.5,
            ball_vx: dir * 30.0,
            ball_vy: rng.random_range(-15.0..15.0),
            left_y: h * 0.5,
            right_y: h * 0.5,
            paddle_h: (h * 0.2).max(4.0),
            left_score: 0,
            right_score: 0,
            serve_timer: 0.0,
            rng: rand::rng(),
        }
    }

    fn serve(&mut self, w: f64, h: f64) {
        self.ball_x = w * 0.5;
        self.ball_y = h * 0.5;
        let dir: f64 = if self.rng.random_range(0u8..2) == 0 {
            1.0
        } else {
            -1.0
        };
        self.ball_vx = dir * 30.0;
        self.ball_vy = self.rng.random_range(-15.0..15.0);
        self.serve_timer = 0.5;
    }
}

impl Animation for Pong {
    fn name(&self) -> &str {
        "pong"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let paddle_w = 2.0;
        let left_x = 3.0;
        let right_x = w - 3.0 - paddle_w;
        self.paddle_h = (h * 0.2).max(4.0);
        let half_paddle = self.paddle_h * 0.5;

        // Serve delay
        if self.serve_timer > 0.0 {
            self.serve_timer -= dt;
            // Still draw everything, just don't move ball
        } else {
            // Move ball
            self.ball_x += self.ball_vx * dt;
            self.ball_y += self.ball_vy * dt;

            // Bounce off top/bottom
            if self.ball_y <= 0.0 {
                self.ball_y = 0.0;
                self.ball_vy = self.ball_vy.abs();
            }
            if self.ball_y >= h - 1.0 {
                self.ball_y = h - 1.0;
                self.ball_vy = -self.ball_vy.abs();
            }

            // Left paddle collision
            if self.ball_vx < 0.0
                && self.ball_x <= left_x + paddle_w
                && self.ball_x >= left_x
                && (self.ball_y - self.left_y).abs() < half_paddle
            {
                self.ball_vx = self.ball_vx.abs() * 1.05;
                let offset = (self.ball_y - self.left_y) / half_paddle;
                self.ball_vy += offset * 15.0;
                self.ball_x = left_x + paddle_w + 0.1;
            }

            // Right paddle collision
            if self.ball_vx > 0.0
                && self.ball_x >= right_x
                && self.ball_x <= right_x + paddle_w
                && (self.ball_y - self.right_y).abs() < half_paddle
            {
                self.ball_vx = -self.ball_vx.abs() * 1.05;
                let offset = (self.ball_y - self.right_y) / half_paddle;
                self.ball_vy += offset * 15.0;
                self.ball_x = right_x - 0.1;
            }

            // Clamp ball speed
            self.ball_vx = self.ball_vx.clamp(-80.0, 80.0);
            self.ball_vy = self.ball_vy.clamp(-40.0, 40.0);

            // Score
            if self.ball_x < 0.0 {
                self.right_score += 1;
                if self.right_score >= 11 {
                    self.left_score = 0;
                    self.right_score = 0;
                }
                self.serve(w, h);
            }
            if self.ball_x >= w {
                self.left_score += 1;
                if self.left_score >= 11 {
                    self.left_score = 0;
                    self.right_score = 0;
                }
                self.serve(w, h);
            }
        }

        // AI paddles: track ball with slight lag
        let ai_speed = 35.0;
        let left_diff = self.ball_y - self.left_y;
        self.left_y += left_diff.clamp(-ai_speed * dt, ai_speed * dt);
        self.left_y = self.left_y.clamp(half_paddle, h - half_paddle);

        let right_diff = self.ball_y - self.right_y;
        self.right_y += right_diff.clamp(-ai_speed * dt, ai_speed * dt);
        self.right_y = self.right_y.clamp(half_paddle, h - half_paddle);

        // Render
        canvas.clear();

        // Center line
        let cx = (w * 0.5) as usize;
        for y in 0..canvas.height {
            if y % 3 != 0 && cx < canvas.width {
                canvas.set_colored(cx, y, 0.2, 100, 100, 100);
            }
        }

        // Left paddle
        let lx = left_x as usize;
        let l_top = (self.left_y - half_paddle).max(0.0) as usize;
        let l_bot = (self.left_y + half_paddle).min(h) as usize;
        for y in l_top..l_bot {
            for dx in 0..(paddle_w as usize) {
                let px = lx + dx;
                if px < canvas.width && y < canvas.height {
                    canvas.set_colored(px, y, 0.9, 100, 200, 255);
                }
            }
        }

        // Right paddle
        let rx = right_x as usize;
        let r_top = (self.right_y - half_paddle).max(0.0) as usize;
        let r_bot = (self.right_y + half_paddle).min(h) as usize;
        for y in r_top..r_bot {
            for dx in 0..(paddle_w as usize) {
                let px = rx + dx;
                if px < canvas.width && y < canvas.height {
                    canvas.set_colored(px, y, 0.9, 255, 100, 100);
                }
            }
        }

        // Ball
        let bx = self.ball_x as usize;
        let by = self.ball_y as usize;
        if bx < canvas.width && by < canvas.height {
            canvas.set_colored(bx, by, 1.0, 255, 255, 255);
        }
        // Ball glow
        for &(ox, oy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let gx = (self.ball_x + ox as f64) as usize;
            let gy = (self.ball_y + oy as f64) as usize;
            if gx < canvas.width && gy < canvas.height {
                canvas.set_colored(gx, gy, 0.4, 200, 200, 200);
            }
        }

        // Score display (simple dots in top area)
        let score_y = 2_usize;
        let left_score_x = (w * 0.3) as usize;
        let right_score_x = (w * 0.7) as usize;
        for i in 0..self.left_score.min(10) as usize {
            let sx = left_score_x + i * 2;
            if sx < canvas.width && score_y < canvas.height {
                canvas.set_colored(sx, score_y, 0.8, 100, 200, 255);
            }
        }
        for i in 0..self.right_score.min(10) as usize {
            let sx = right_score_x + i * 2;
            if sx < canvas.width && score_y < canvas.height {
                canvas.set_colored(sx, score_y, 0.8, 255, 100, 100);
            }
        }
    }
}
