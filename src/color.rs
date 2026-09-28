use crossterm::style::Color;

/// Map a 256-color palette index to its RGB triple (VGA values for 0-15,
/// 6x6x6 cube for 16-231, grayscale ramp for 232-255).
pub fn ansi256_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=7 => {
            const C: [(u8, u8, u8); 8] = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
            ];
            C[idx as usize]
        }
        8..=15 => {
            const C: [(u8, u8, u8); 8] = [
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];
            C[(idx - 8) as usize]
        }
        16..=231 => {
            let n = idx - 16;
            let b_val = n % 6;
            let g_val = (n / 6) % 6;
            let r_val = n / 36;
            const LEVEL: [u8; 6] = [0, 95, 135, 175, 215, 255];
            (
                LEVEL[r_val as usize],
                LEVEL[g_val as usize],
                LEVEL[b_val as usize],
            )
        }
        _ => {
            let v = 8 + 10 * (idx as u32 - 232);
            (v as u8, v as u8, v as u8)
        }
    }
}

/// Map any crossterm color to an RGB triple. Named colors resolve through
/// their ANSI 0-15 equivalents; `Reset` and unknown variants fall back to
/// black (the same default `Cell`'s `None` colors mean).
pub fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::AnsiValue(v) => ansi256_to_rgb(v),
        Color::Black => ansi256_to_rgb(0),
        Color::DarkRed => ansi256_to_rgb(1),
        Color::DarkGreen => ansi256_to_rgb(2),
        Color::DarkYellow => ansi256_to_rgb(3),
        Color::DarkBlue => ansi256_to_rgb(4),
        Color::DarkMagenta => ansi256_to_rgb(5),
        Color::DarkCyan => ansi256_to_rgb(6),
        Color::Grey => ansi256_to_rgb(7),
        Color::DarkGrey => ansi256_to_rgb(8),
        Color::Red => ansi256_to_rgb(9),
        Color::Green => ansi256_to_rgb(10),
        Color::Yellow => ansi256_to_rgb(11),
        Color::Blue => ansi256_to_rgb(12),
        Color::Magenta => ansi256_to_rgb(13),
        Color::Cyan => ansi256_to_rgb(14),
        Color::White => ansi256_to_rgb(15),
        _ => (0, 0, 0),
    }
}

pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = h.rem_euclid(1.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_one_wraps_to_zero() {
        assert_eq!(hsv_to_rgb(1.0, 1.0, 1.0), hsv_to_rgb(0.0, 1.0, 1.0));
    }

    #[test]
    fn negative_quarter_hue_wraps_to_three_quarters() {
        assert_eq!(hsv_to_rgb(-0.25, 1.0, 1.0), hsv_to_rgb(0.75, 1.0, 1.0));
    }

    #[test]
    fn known_primaries() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), (255, 0, 0));
        assert_eq!(hsv_to_rgb(1.0 / 3.0, 1.0, 1.0), (0, 255, 0));
        assert_eq!(hsv_to_rgb(2.0 / 3.0, 1.0, 1.0), (0, 0, 255));
    }

    #[test]
    fn color_to_rgb_direct_and_named() {
        assert_eq!(color_to_rgb(Color::Rgb { r: 1, g: 2, b: 3 }), (1, 2, 3));
        assert_eq!(color_to_rgb(Color::AnsiValue(196)), (255, 0, 0));
        assert_eq!(color_to_rgb(Color::White), (255, 255, 255));
        assert_eq!(color_to_rgb(Color::Reset), (0, 0, 0));
    }
}
