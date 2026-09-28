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
}
