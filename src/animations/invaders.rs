use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

struct Alien {
    x: f64,
    y: f64,
    alive: bool,
    kind: u8,
}

struct Bullet {
    x: f64,
    y: f64,
    vy: f64,
    is_player: bool,
}

/// Space Invaders attract mode demo with aliens marching and shooting
pub struct Invaders {
    width: usize,
    height: usize,
    aliens: Vec<Alien>,
    bullets: Vec<Bullet>,
    player_x: f64,
    player_target: f64,
    move_dir: f64,
    move_timer: f64,
    move_interval: f64,
    shoot_timer: f64,
    alien_shoot_timer: f64,
    wave: usize,
}

impl Invaders {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let mut inv = Invaders {
            width,
            height,
            aliens: Vec::new(),
            bullets: Vec::new(),
            player_x: width as f64 * 0.5,
            player_target: width as f64 * 0.5,
            move_dir: 1.0,
            move_timer: 0.0,
            move_interval: 0.5,
            shoot_timer: 0.0,
            alien_shoot_timer: 0.0,
            wave: 0,
        };
        inv.spawn_wave();
        inv
    }

    fn spawn_wave(&mut self) {
        self.aliens.clear();
        self.bullets.clear();
        let cols = (self.width / 6).clamp(5, 11);
        let rows = 5;
        let spacing_x = self.width as f64 / (cols + 1) as f64;
        let spacing_y = (self.height as f64 * 0.05).max(3.0);

        for row in 0..rows {
            for col in 0..cols {
                self.aliens.push(Alien {
                    x: spacing_x * (col + 1) as f64,
                    y: spacing_y * (row + 1) as f64 + 2.0,
                    alive: true,
                    kind: row as u8,
                });
            }
        }
        self.wave += 1;
        self.move_interval = (0.5 - self.wave as f64 * 0.05).max(0.1);
    }
}

impl Animation for Invaders {
    fn name(&self) -> &str {
        "invaders"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;
        let w = self.width as f64;
        let h = self.height as f64;
        let player_y = h - 4.0;

        // Move aliens
        self.move_timer += dt;
        if self.move_timer >= self.move_interval {
            self.move_timer = 0.0;

            // Check if any alien hit the edge
            let mut hit_edge = false;
            for alien in &self.aliens {
                if !alien.alive {
                    continue;
                }
                if (alien.x + self.move_dir * 3.0) >= w - 2.0
                    || (alien.x + self.move_dir * 3.0) <= 2.0
                {
                    hit_edge = true;
                    break;
                }
            }

            if hit_edge {
                self.move_dir = -self.move_dir;
                for alien in &mut self.aliens {
                    alien.y += 2.0;
                }
            } else {
                for alien in &mut self.aliens {
                    alien.x += self.move_dir * 3.0;
                }
            }
        }

        // Player AI: move toward nearest alien column
        let alive_aliens: Vec<&Alien> = self.aliens.iter().filter(|a| a.alive).collect();
        if let Some(target) = alive_aliens.iter().min_by(|a, b| {
            let da = (a.x - self.player_x).abs();
            let db = (b.x - self.player_x).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            self.player_target = target.x;
        }

        // Move player toward target
        let diff = self.player_target - self.player_x;
        self.player_x += diff.clamp(-30.0, 30.0) * dt;
        self.player_x = self.player_x.clamp(3.0, w - 3.0);

        // Player shoots
        self.shoot_timer += dt;
        if self.shoot_timer >= 0.4 {
            self.shoot_timer = 0.0;
            self.bullets.push(Bullet {
                x: self.player_x,
                y: player_y - 1.0,
                vy: -40.0,
                is_player: true,
            });
        }

        // Aliens shoot
        self.alien_shoot_timer += dt;
        if self.alien_shoot_timer >= 1.0 && !alive_aliens.is_empty() {
            self.alien_shoot_timer = 0.0;
            let shooter_idx = rng.random_range(0..alive_aliens.len());
            let shooter = alive_aliens[shooter_idx];
            self.bullets.push(Bullet {
                x: shooter.x,
                y: shooter.y + 2.0,
                vy: 20.0,
                is_player: false,
            });
        }

        // Update bullets
        for bullet in &mut self.bullets {
            bullet.y += bullet.vy * dt;
        }

        // Check collisions: player bullets hit aliens
        let mut kill_list = Vec::new();
        for bullet in &mut self.bullets {
            if !bullet.is_player {
                continue;
            }
            for (ai, alien) in self.aliens.iter().enumerate() {
                if !alien.alive {
                    continue;
                }
                if (bullet.x - alien.x).abs() < 2.5 && (bullet.y - alien.y).abs() < 2.0 {
                    kill_list.push(ai);
                    bullet.y = -100.0; // Remove bullet
                    break;
                }
            }
        }
        for ai in kill_list {
            self.aliens[ai].alive = false;
        }

        // Remove off-screen bullets
        self.bullets.retain(|b| b.y > -5.0 && b.y < h + 5.0);

        // Check if wave is cleared
        if self.aliens.iter().all(|a| !a.alive) {
            self.spawn_wave();
        }

        // Check if aliens reached the bottom
        let lowest = self
            .aliens
            .iter()
            .filter(|a| a.alive)
            .map(|a| a.y)
            .fold(0.0, f64::max);
        if lowest >= player_y - 2.0 {
            self.spawn_wave();
        }

        // Render
        canvas.clear();

        // Draw aliens
        for alien in &self.aliens {
            if !alien.alive {
                continue;
            }
            let (r, g, b) = match alien.kind {
                0 => (255, 100, 100),
                1 => (255, 180, 50),
                2 => (100, 255, 100),
                3 => (100, 200, 255),
                _ => (200, 100, 255),
            };

            // Draw alien as a small sprite (3x2 pixels)
            let ax = alien.x as usize;
            let ay = alien.y as usize;
            for dy in 0..2_usize {
                for dx in 0..3_usize {
                    let px = ax.wrapping_add(dx).wrapping_sub(1);
                    let py = ay + dy;
                    if px < canvas.width && py < canvas.height {
                        let is_body = !(dy == 0 && (dx == 0 || dx == 2));
                        let brightness = if is_body { 0.9 } else { 0.6 };
                        canvas.set_colored(px, py, brightness, r, g, b);
                    }
                }
            }
        }

        // Draw player
        let px = self.player_x as usize;
        let py = player_y as usize;
        for dx in 0..5_usize {
            let x = px.wrapping_add(dx).wrapping_sub(2);
            if x < canvas.width && py < canvas.height {
                let is_cannon = dx == 2;
                let y = if is_cannon { py.saturating_sub(1) } else { py };
                canvas.set_colored(x, y, 0.9, 50, 255, 50);
                if is_cannon && y > 0 {
                    canvas.set_colored(x, y, 1.0, 100, 255, 100);
                }
            }
        }

        // Draw bullets
        for bullet in &self.bullets {
            let bx = bullet.x as usize;
            let by = bullet.y as usize;
            if bx < canvas.width && by < canvas.height {
                if bullet.is_player {
                    canvas.set_colored(bx, by, 1.0, 255, 255, 100);
                } else {
                    canvas.set_colored(bx, by, 1.0, 255, 100, 100);
                }
            }
        }
    }
}
