use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct Organelle {
    angle: f64,
    dist: f64, // fraction of cell radius
    speed: f64,
    size: f64,
    hue_shift: f64,
}

struct Cell {
    x: f64,
    y: f64,
    radius: f64,
    phase: f64,
    hue: f64,
    split_progress: f64,
    splitting: bool,
    elongation: f64, // 0=circle, >0=oval
    elong_angle: f64,
    organelles: Vec<Organelle>,
    trail: Vec<(f64, f64, f64)>, // x, y, age
    membrane_wobble: f64,
}

/// Petri dish under microscope — glowing cells with organelles, membranes, and trails
pub struct Cells {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    max_cells: usize,
    fluid_time: f64,
    rng: rand::rngs::ThreadRng,
}

impl Cells {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let mut rng = rand::rng();
        let initial = 5;
        let cells = (0..initial)
            .map(|_| make_cell(&mut rng, width, height))
            .collect();

        Cells {
            width,
            height,
            cells,
            max_cells: (35.0 * scale) as usize,
            fluid_time: 0.0,
            rng: rand::rng(),
        }
    }
}

fn make_cell(rng: &mut impl rand::RngExt, width: usize, height: usize) -> Cell {
    let radius = rng.random_range(5.0..14.0);
    let num_org = rng.random_range(2..6);
    let organelles = (0..num_org)
        .map(|_| Organelle {
            angle: rng.random_range(0.0..std::f64::consts::TAU),
            dist: rng.random_range(0.2..0.7),
            speed: rng.random_range(-2.0..2.0),
            size: rng.random_range(0.8..2.0),
            hue_shift: rng.random_range(-0.15..0.15),
        })
        .collect();

    Cell {
        x: rng.random_range(width as f64 * 0.2..width as f64 * 0.8),
        y: rng.random_range(height as f64 * 0.2..height as f64 * 0.8),
        radius,
        phase: rng.random_range(0.0..std::f64::consts::TAU),
        hue: rng.random_range(0.0..1.0),
        split_progress: 0.0,
        splitting: false,
        elongation: rng.random_range(0.0..0.35),
        elong_angle: rng.random_range(0.0..std::f64::consts::TAU),
        organelles,
        trail: Vec::new(),
        membrane_wobble: rng.random_range(0.5..1.5),
    }
}

impl Cells {
    fn reset(&mut self) {
        self.cells.clear();
        for _ in 0..5 {
            self.cells
                .push(make_cell(&mut self.rng, self.width, self.height));
        }
    }
}

impl Animation for Cells {
    fn name(&self) -> &str {
        "cells"
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.width = canvas.width;
        self.height = canvas.height;
        let w = self.width as f64;
        let h = self.height as f64;
        self.fluid_time += dt;

        if self.cells.len() > self.max_cells {
            self.reset();
        }

        // Trigger splitting
        for cell in &mut self.cells {
            if !cell.splitting && cell.radius > 8.0 && self.rng.random_range(0.0..1.0) < 0.004 {
                cell.splitting = true;
                cell.split_progress = 0.0;
            }
        }

        // Update cells
        let mut new_cells = Vec::new();
        for cell in &mut self.cells {
            // Brownian + drift movement
            let drift_x =
                (time * 0.3 + cell.phase).cos() * 3.0 + (time * 0.7 + cell.phase * 2.3).sin() * 1.5;
            let drift_y =
                (time * 0.4 + cell.phase).sin() * 2.5 + (time * 0.6 + cell.phase * 1.7).cos() * 1.2;
            cell.x += (drift_x + self.rng.random_range(-1.0..1.0)) * dt;
            cell.y += (drift_y + self.rng.random_range(-0.8..0.8)) * dt;
            cell.phase += dt * 0.5;
            cell.elong_angle += dt * 0.2;

            // Soft wall repulsion — keep cells well within bounds
            let margin = cell.radius + 4.0;
            let wall_force = 5.0;
            if cell.x < margin {
                cell.x += (margin - cell.x) * wall_force * dt;
            }
            if cell.x > w - margin {
                cell.x -= (cell.x - (w - margin)) * wall_force * dt;
            }
            if cell.y < margin {
                cell.y += (margin - cell.y) * wall_force * dt;
            }
            if cell.y > h - margin {
                cell.y -= (cell.y - (h - margin)) * wall_force * dt;
            }
            cell.x = cell.x.clamp(2.0, w - 2.0);
            cell.y = cell.y.clamp(2.0, h - 2.0);

            // Update organelles
            for org in &mut cell.organelles {
                org.angle += org.speed * dt;
            }

            // Trail
            cell.trail.push((cell.x, cell.y, 0.0));
            for t in &mut cell.trail {
                t.2 += dt;
            }
            cell.trail.retain(|t| t.2 < 3.0);

            if cell.splitting {
                cell.split_progress += dt * 0.35;
                if cell.split_progress >= 1.0 {
                    let split_angle = cell.elong_angle;
                    let new_radius = cell.radius * 0.7;
                    let offset = new_radius * 1.5;

                    let mut daughter = make_cell(&mut self.rng, w as usize, h as usize);
                    daughter.x = cell.x + split_angle.cos() * offset;
                    daughter.y = cell.y + split_angle.sin() * offset;
                    daughter.radius = new_radius;
                    daughter.hue = (cell.hue + self.rng.random_range(-0.08..0.08)).fract().abs();
                    new_cells.push(daughter);

                    cell.x -= split_angle.cos() * offset;
                    cell.y -= split_angle.sin() * offset;
                    cell.radius = new_radius;
                    cell.splitting = false;
                    cell.split_progress = 0.0;
                }
            } else {
                cell.radius = (cell.radius + dt * 0.2).min(16.0);
            }
        }
        self.cells.extend(new_cells);

        // Repulsion
        let positions: Vec<(f64, f64, f64)> =
            self.cells.iter().map(|c| (c.x, c.y, c.radius)).collect();
        for (i, cell) in self.cells.iter_mut().enumerate() {
            for (j, &(ox, oy, or)) in positions.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dx = cell.x - ox;
                let dy = cell.y - oy;
                let dist = (dx * dx + dy * dy).sqrt();
                let min_dist = cell.radius + or + 2.0;
                if dist < min_dist && dist > 0.1 {
                    let push = (min_dist - dist) * 0.8 * dt;
                    cell.x += dx / dist * push;
                    cell.y += dy / dist * push;
                }
            }
        }

        canvas.clear();

        // Fluid background — subtle flowing medium
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64 * 0.04;
                let fy = y as f64 * 0.06;
                let t = self.fluid_time * 0.15;
                let n = ((fx + t).sin() * (fy - t * 0.7).cos()
                    + (fx * 0.7 - t * 0.5).cos() * (fy * 1.3 + t).sin())
                    * 0.5;
                let v = (n * 0.5 + 0.5).clamp(0.0, 1.0);
                if v > 0.4 {
                    let b = (v - 0.4) * 0.08;
                    canvas.set_colored(x, y, b, 15, 25, 40);
                }
            }
        }

        // Microscope vignette
        let vcx = w * 0.5;
        let vcy = h * 0.5;
        let vmax = (vcx * vcx + vcy * vcy).sqrt();

        // Draw trails
        for cell in &self.cells {
            let (cr, cg, cb) = hsv_to_rgb(cell.hue, 0.3, 0.3);
            for &(tx, ty, age) in &cell.trail {
                let fade = (1.0 - age / 3.0).max(0.0) * 0.1;
                if fade > 0.01 {
                    let px = tx as usize;
                    let py = ty as usize;
                    if px < canvas.width && py < canvas.height {
                        canvas.set_colored(px, py, fade, cr, cg, cb);
                    }
                }
            }
        }

        // Draw cells
        for cell in &self.cells {
            let pulse = 1.0 + (cell.phase * 1.5).sin() * 0.08;
            let base_r = cell.radius * pulse;
            let (cr, cg, cb) = hsv_to_rgb(cell.hue, 0.4, 0.7);

            if cell.splitting {
                let t = cell.split_progress;
                let sep = t * cell.radius;
                let angle = cell.elong_angle;
                for lobe in 0..2 {
                    let sign = if lobe == 0 { -1.0 } else { 1.0 };
                    let lx = cell.x + angle.cos() * sep * sign;
                    let ly = cell.y + angle.sin() * sep * sign;
                    let lr = base_r * (1.0 - t * 0.25);
                    draw_membrane_cell(
                        canvas,
                        lx,
                        ly,
                        lr,
                        0.0,
                        0.0,
                        cr,
                        cg,
                        cb,
                        cell.membrane_wobble,
                        time,
                    );
                }
                // Bridge between lobes
                if t < 0.85 {
                    let bridge_alpha = (1.0 - t / 0.85) * 0.4;
                    let steps = (sep * 3.0) as usize;
                    for s in 0..steps.max(1) {
                        let frac = s as f64 / steps as f64;
                        let bx =
                            (cell.x - angle.cos() * sep + angle.cos() * sep * 2.0 * frac) as usize;
                        let by =
                            (cell.y - angle.sin() * sep + angle.sin() * sep * 2.0 * frac) as usize;
                        if bx < canvas.width && by < canvas.height {
                            canvas.set_colored(bx, by, bridge_alpha, cr, cg, cb);
                        }
                    }
                }
            } else {
                draw_membrane_cell(
                    canvas,
                    cell.x,
                    cell.y,
                    base_r,
                    cell.elongation,
                    cell.elong_angle,
                    cr,
                    cg,
                    cb,
                    cell.membrane_wobble,
                    time,
                );

                // Draw organelles
                for org in &cell.organelles {
                    let ox = cell.x + (org.angle.cos() * org.dist * base_r);
                    let oy = cell.y + (org.angle.sin() * org.dist * base_r);
                    let (or, og, ob) =
                        hsv_to_rgb((cell.hue + org.hue_shift).fract().abs(), 0.6, 0.9);
                    let orad = org.size;
                    let min_x = (ox - orad).max(0.0) as usize;
                    let max_x = (ox + orad + 1.0).min(canvas.width as f64) as usize;
                    let min_y = (oy - orad).max(0.0) as usize;
                    let max_y = (oy + orad + 1.0).min(canvas.height as f64) as usize;
                    for py in min_y..max_y {
                        for px in min_x..max_x {
                            let dx = px as f64 - ox;
                            let dy = py as f64 - oy;
                            if dx * dx + dy * dy < orad * orad {
                                canvas.set_colored(px, py, 0.7, or, og, ob);
                            }
                        }
                    }
                }

                // Nucleus — darker center blob
                let nuc_r = base_r * 0.25;
                let (nr, ng, nb) = hsv_to_rgb(cell.hue, 0.7, 0.5);
                let min_x = (cell.x - nuc_r).max(0.0) as usize;
                let max_x = (cell.x + nuc_r + 1.0).min(canvas.width as f64) as usize;
                let min_y = (cell.y - nuc_r).max(0.0) as usize;
                let max_y = (cell.y + nuc_r + 1.0).min(canvas.height as f64) as usize;
                for py in min_y..max_y {
                    for px in min_x..max_x {
                        let dx = px as f64 - cell.x;
                        let dy = py as f64 - cell.y;
                        let d2 = dx * dx + dy * dy;
                        if d2 < nuc_r * nuc_r {
                            let edge = 1.0 - (d2 / (nuc_r * nuc_r)).sqrt();
                            canvas.set_colored(px, py, 0.5 + edge * 0.3, nr, ng, nb);
                        }
                    }
                }
            }
        }

        // Petri dish border + vignette
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let dx = x as f64 - vcx;
                let dy = (y as f64 - vcy) * 1.5;
                let dist = (dx * dx + dy * dy).sqrt() / vmax;

                // Dish rim — faint circular edge
                if dist > 0.82 && dist < 0.90 {
                    let rim = 1.0 - ((dist - 0.86).abs() / 0.04).min(1.0);
                    if rim > 0.05 {
                        canvas.set_colored(x, y, rim * 0.25, 80, 100, 120);
                    }
                }

                // Vignette darkening
                if dist > 0.7 {
                    let darken = ((dist - 0.7) / 0.3).clamp(0.0, 1.0);
                    if darken > 0.05 {
                        canvas.set_colored(x, y, darken * 0.6, 0, 0, 0);
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_membrane_cell(
    canvas: &mut Canvas,
    cx: f64,
    cy: f64,
    radius: f64,
    elongation: f64,
    elong_angle: f64,
    r: u8,
    g: u8,
    b: u8,
    wobble_freq: f64,
    time: f64,
) {
    let scan_r = radius * (1.0 + elongation) + 2.0;
    let min_x = (cx - scan_r).max(0.0) as usize;
    let max_x = (cx + scan_r + 1.0).min(canvas.width as f64) as usize;
    let min_y = (cy - scan_r).max(0.0) as usize;
    let max_y = (cy + scan_r + 1.0).min(canvas.height as f64) as usize;

    for y in min_y..max_y {
        for x in min_x..max_x {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;

            // Rotate into elongation frame
            let cos_a = elong_angle.cos();
            let sin_a = elong_angle.sin();
            let rx = dx * cos_a + dy * sin_a;
            let ry = -dx * sin_a + dy * cos_a;

            // Ellipse: stretch along x
            let sx = 1.0 / (1.0 + elongation);
            let sy = 1.0 + elongation * 0.3;
            let norm_dist = ((rx * sx) * (rx * sx) + (ry * sy) * (ry * sy)).sqrt() / radius;

            // Membrane wobble
            let angle = dy.atan2(dx);
            let wobble = (angle * 5.0 * wobble_freq + time * 2.0).sin() * 0.06;
            let norm_dist = norm_dist + wobble;

            if norm_dist > 1.05 {
                continue;
            }

            if norm_dist > 0.85 {
                // Membrane edge — bright glow
                let edge = 1.0 - ((norm_dist - 0.85) / 0.2).clamp(0.0, 1.0);
                let glow = edge * 0.9;
                // Brighter, more saturated membrane
                let mr = (r as f64 * 1.3).min(255.0) as u8;
                let mg = (g as f64 * 1.3).min(255.0) as u8;
                let mb = (b as f64 * 1.3).min(255.0) as u8;
                canvas.set_colored(x, y, glow, mr, mg, mb);
            } else {
                // Interior — translucent fill
                let interior = 0.15 + (1.0 - norm_dist / 0.85) * 0.15;
                canvas.set_colored(
                    x,
                    y,
                    interior,
                    (r as f64 * 0.7) as u8,
                    (g as f64 * 0.7) as u8,
                    (b as f64 * 0.7) as u8,
                );
            }
        }
    }
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((h % 1.0) + 1.0) % 1.0;
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
