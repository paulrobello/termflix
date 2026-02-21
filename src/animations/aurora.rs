use super::Animation;
use crate::render::Canvas;
use noise::{NoiseFn, Perlin};

/// Aurora borealis effect
pub struct Aurora {
    noise: Perlin,
}

impl Aurora {
    pub fn new() -> Self {
        Aurora {
            noise: Perlin::new(42),
        }
    }
}

impl Animation for Aurora {
    fn name(&self) -> &str {
        "aurora"
    }

    fn update(&mut self, canvas: &mut Canvas, _dt: f64, time: f64) {
        let w = canvas.width as f64;
        let h = canvas.height as f64;
        let t = time * 0.3;

        canvas.clear();

        // Draw faint stars in background
        let star_seed = 12345u64;
        for i in 0..80 {
            let sx = ((star_seed.wrapping_mul(i * 7 + 3)) % canvas.width as u64) as usize;
            let sy = ((star_seed.wrapping_mul(i * 13 + 7)) % (canvas.height as u64 / 3)) as usize;
            let twinkle = (t * 2.0 + i as f64 * 0.7).sin() * 0.5 + 0.5;
            if twinkle > 0.3 {
                let b = (twinkle * 120.0) as u8;
                canvas.set_colored(sx, sy, twinkle * 0.4, b, b, (b as u16 + 30).min(255) as u8);
            }
        }

        for x in 0..canvas.width {
            let fx = x as f64 / w;

            // Multiple curtain layers with distinct colors
            for layer in 0..4 {
                let offset = layer as f64 * 0.8;
                let speed = 0.8 + layer as f64 * 0.2;

                let n1 = self.noise.get([fx * 3.0 + offset, t * speed * 0.5]);
                let n2 = self
                    .noise
                    .get([fx * 7.0 + offset + 10.0, t * speed * 0.3 + 5.0]);
                let curtain_y = h * (0.08 + n1 * 0.2 + n2 * 0.06);
                let curtain_height =
                    h * (0.35 + self.noise.get([fx * 2.0, t * 0.15 + offset]) * 0.2);

                for y in 0..canvas.height {
                    let fy = y as f64;
                    let dist = fy - curtain_y;

                    if dist < 0.0 || dist > curtain_height {
                        continue;
                    }

                    let vert_fade = 1.0 - (dist / curtain_height);
                    let vert_fade = vert_fade * vert_fade;

                    let shimmer = self.noise.get([fx * 12.0, fy * 0.08, t * 2.5 + offset]);
                    let intensity = (vert_fade * (0.5 + shimmer * 0.5)).clamp(0.0, 1.0);

                    if intensity < 0.03 {
                        continue;
                    }

                    // Vivid aurora palette — each layer a different hue
                    let (r, g, b) = match layer {
                        0 => {
                            // Bright green (classic aurora)
                            (
                                (20.0 * intensity) as u8,
                                (180.0 + 75.0 * intensity) as u8,
                                (40.0 + 60.0 * intensity) as u8,
                            )
                        }
                        1 => {
                            // Cyan-teal
                            (
                                (20.0 + 40.0 * intensity) as u8,
                                (140.0 + 115.0 * intensity) as u8,
                                (160.0 + 95.0 * intensity) as u8,
                            )
                        }
                        2 => {
                            // Purple-magenta
                            (
                                (120.0 + 100.0 * intensity) as u8,
                                (20.0 + 50.0 * intensity) as u8,
                                (160.0 + 95.0 * intensity) as u8,
                            )
                        }
                        _ => {
                            // Pink-red (rare top layer)
                            (
                                (180.0 + 75.0 * intensity) as u8,
                                (30.0 + 40.0 * intensity) as u8,
                                (80.0 + 80.0 * intensity) as u8,
                            )
                        }
                    };

                    // Layer blend — brighter layer wins, colors mix via max
                    let idx = y * canvas.width + x;
                    let existing_b = canvas.pixels[idx];
                    if intensity > existing_b {
                        canvas.set_colored(x, y, intensity, r, g, b);
                    } else if intensity > existing_b * 0.5 {
                        // Blend colors when layers overlap
                        let (er, eg, eb) = canvas.colors[idx];
                        let t = intensity / existing_b.max(0.01);
                        let mr = (er as f64 * (1.0 - t) + r as f64 * t) as u8;
                        let mg = (eg as f64 * (1.0 - t) + g as f64 * t) as u8;
                        let mb = (eb as f64 * (1.0 - t) + b as f64 * t) as u8;
                        canvas.set_colored(x, y, existing_b, mr, mg, mb);
                    }
                }
            }
        }
    }
}
