use crate::render::Canvas;
use super::Animation;

/// Animated Sierpinski triangle with zoom
pub struct Sierpinski {
    zoom: f64,
}

impl Sierpinski {
    pub fn new() -> Self {
        Sierpinski { zoom: 1.0 }
    }
}

impl Animation for Sierpinski {
    fn name(&self) -> &str {
        "sierpinski"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let cx = w * 0.5;
        let cy = h * 0.5;

        // Zoom cycles
        self.zoom = 1.0 + (time * 0.3).sin().abs() * 4.0;
        let color_offset = time * 0.2;

        canvas.clear();

        // Sierpinski triangle check using the chaos game property:
        // A point (x,y) in barycentric coords is in the Sierpinski triangle
        // if at no level of recursion do both coordinates have a 1-bit
        // in the same position.
        //
        // We use the pixel-based approach: for each pixel, check if it's in the set

        let size = w.min(h) * 0.9 * self.zoom;
        let half = size * 0.5;

        // Triangle vertices (equilateral)
        let ax = cx;
        let ay = cy - half * 0.866;
        let bx = cx - half;
        let by = cy + half * 0.433;
        let _cx_t = cx + half;
        let _cy_t = cy + half * 0.433;

        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let fx = x as f64;
                let fy = y as f64;

                // Convert to barycentric-like coordinates relative to triangle
                // Use the address method: convert pixel to position in [0,1]x[0,1] space
                let rel_x = (fx - (cx - half)) / size;
                let rel_y = (fy - (cy - half * 0.866)) / (size * 0.866);

                if rel_x < 0.0 || rel_x >= 1.0 || rel_y < 0.0 || rel_y >= 1.0 {
                    continue;
                }

                // Use iterative subdivision to check Sierpinski membership
                let mut px = rel_x;
                let mut py = rel_y;
                let mut in_set = true;
                let max_depth = 8;
                let mut depth = 0;

                for d in 0..max_depth {
                    // Scale coordinates to [0, 2]
                    let sx = px * 2.0;
                    let sy = py * 2.0;

                    if sy > 1.0 {
                        // Top region
                        px = sx - 0.5;
                        py = sy - 1.0;
                        if px < 0.0 || px > 1.0 {
                            in_set = false;
                            break;
                        }
                    } else if sx < 1.0 {
                        // Bottom-left
                        px = sx;
                        py = sy;
                    } else {
                        // Bottom-right
                        px = sx - 1.0;
                        py = sy;
                    }

                    // Check if we're in the empty middle triangle
                    // The middle triangle is roughly where sx in [0.5, 1.5] and sy in [0, 1]
                    // and above the line from (0.5, 0) to (1.0, 1.0) and (1.0, 1.0) to (1.5, 0)
                    if sy <= 1.0 && sx >= 0.5 && sx <= 1.5 {
                        let mid = sx - 0.5;
                        if mid <= 1.0 && sy < 1.0 - (mid - 0.5).abs() * 2.0 {
                            // In the gap
                            if sy > 0.0 {
                                in_set = false;
                                break;
                            }
                        }
                    }

                    depth = d;
                }

                if in_set {
                    let hue = ((depth as f64 / max_depth as f64) + color_offset).fract();
                    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);
                    let brightness = 0.5 + (depth as f64 / max_depth as f64) * 0.5;
                    canvas.set_colored(x, y, brightness, r, g, b);
                }
            }
        }

        // Alternative: draw using recursive triangles for cleaner result
        draw_sierpinski_recursive(
            canvas,
            ax, ay, bx, by, _cx_t, _cy_t,
            6, 0, color_offset,
        );

        let _ = self.zoom;
    }
}

fn draw_sierpinski_recursive(
    canvas: &mut Canvas,
    ax: f64, ay: f64,
    bx: f64, by: f64,
    cx: f64, cy: f64,
    depth: usize,
    current_depth: usize,
    color_offset: f64,
) {
    if depth == 0 {
        // Fill triangle
        fill_triangle(canvas, ax, ay, bx, by, cx, cy, current_depth, color_offset);
        return;
    }

    // Midpoints
    let mab_x = (ax + bx) * 0.5;
    let mab_y = (ay + by) * 0.5;
    let mbc_x = (bx + cx) * 0.5;
    let mbc_y = (by + cy) * 0.5;
    let mac_x = (ax + cx) * 0.5;
    let mac_y = (ay + cy) * 0.5;

    // Three sub-triangles (skip the middle one)
    draw_sierpinski_recursive(canvas, ax, ay, mab_x, mab_y, mac_x, mac_y, depth - 1, current_depth + 1, color_offset);
    draw_sierpinski_recursive(canvas, mab_x, mab_y, bx, by, mbc_x, mbc_y, depth - 1, current_depth + 1, color_offset);
    draw_sierpinski_recursive(canvas, mac_x, mac_y, mbc_x, mbc_y, cx, cy, depth - 1, current_depth + 1, color_offset);
}

fn fill_triangle(
    canvas: &mut Canvas,
    ax: f64, ay: f64,
    bx: f64, by: f64,
    cx: f64, cy: f64,
    depth: usize,
    color_offset: f64,
) {
    let min_x = ax.min(bx).min(cx).max(0.0) as usize;
    let max_x = ax.max(bx).max(cx).min(canvas.width as f64 - 1.0) as usize;
    let min_y = ay.min(by).min(cy).max(0.0) as usize;
    let max_y = ay.max(by).max(cy).min(canvas.height as f64 - 1.0) as usize;

    let hue = (depth as f64 * 0.12 + color_offset).fract();
    let (r, g, b) = hsv_to_rgb(hue, 0.85, 0.95);
    let brightness = 0.6 + (depth as f64 * 0.05).min(0.4);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_triangle(x as f64, y as f64, ax, ay, bx, by, cx, cy) {
                canvas.set_colored(x, y, brightness, r, g, b);
            }
        }
    }
}

fn point_in_triangle(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> bool {
    let d1 = sign(px, py, ax, ay, bx, by);
    let d2 = sign(px, py, bx, by, cx, cy);
    let d3 = sign(px, py, cx, cy, ax, ay);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn sign(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2)
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
