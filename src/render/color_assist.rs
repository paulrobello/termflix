//! Colorblind-safe color assist: remap palettes + LMS daltonization.
//! Pure functions of an RGB triple — deterministic, unit-testable.

/// Rec.601 luma of an RGB triple, normalized to `0.0..=1.0`.
pub fn luminance(rgb: (u8, u8, u8)) -> f64 {
    (0.299 * rgb.0 as f64 + 0.587 * rgb.1 as f64 + 0.114 * rgb.2 as f64) / 255.0
}

/// Colorblind-safe remap palettes (perceptually uniform gradients).
/// Stop values are sampled from the canonical published tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    Viridis,
    Magma,
    Inferno,
    Plasma,
    OkabeIto,
}

impl Palette {
    /// Parse a kebab-case name (`"viridis"`, `"okabe-ito"`, …). Case-sensitive.
    pub fn from_name(s: &str) -> Option<Palette> {
        Some(match s {
            "viridis" => Palette::Viridis,
            "magma" => Palette::Magma,
            "inferno" => Palette::Inferno,
            "plasma" => Palette::Plasma,
            "okabe-ito" => Palette::OkabeIto,
            _ => return None,
        })
    }

    /// The kebab-case name (inverse of `from_name`).
    pub fn name(&self) -> &'static str {
        match self {
            Palette::Viridis => "viridis",
            Palette::Magma => "magma",
            Palette::Inferno => "inferno",
            Palette::Plasma => "plasma",
            Palette::OkabeIto => "okabe-ito",
        }
    }

    /// Gradient stops as `(r, g, b)` evenly spaced over `t = 0.0..=1.0`.
    fn stops(&self) -> &'static [(u8, u8, u8)] {
        match self {
            Palette::Viridis => &[
                (68, 1, 84),
                (59, 82, 139),
                (33, 145, 140),
                (94, 201, 98),
                (253, 231, 37),
            ],
            Palette::Magma => &[
                (0, 0, 4),
                (81, 18, 124),
                (183, 55, 121),
                (252, 138, 101),
                (252, 253, 191),
            ],
            Palette::Inferno => &[
                (0, 0, 4),
                (87, 16, 110),
                (188, 54, 62),
                (248, 142, 24),
                (252, 255, 164),
            ],
            Palette::Plasma => &[
                (13, 8, 135),
                (126, 3, 168),
                (203, 70, 121),
                (248, 149, 64),
                (240, 249, 33),
            ],
            Palette::OkabeIto => &[
                (230, 159, 0),
                (86, 180, 233),
                (0, 158, 115),
                (240, 228, 66),
                (0, 114, 178),
                (213, 94, 0),
                (204, 121, 167),
                (0, 0, 0),
            ],
        }
    }

    /// Sample the gradient at `t` (clamped to `0.0..=1.0`), linear interpolation.
    pub fn sample(&self, t: f64) -> (u8, u8, u8) {
        let stops = self.stops();
        let t = t.clamp(0.0, 1.0);
        if stops.len() == 1 {
            return stops[0];
        }
        let scaled = t * (stops.len() - 1) as f64;
        let i = scaled.floor() as usize;
        let i = i.min(stops.len() - 2);
        let f = scaled - i as f64;
        let (r0, g0, b0) = stops[i];
        let (r1, g1, b1) = stops[i + 1];
        let lerp = |a: u8, b: u8| {
            (a as f64 + (b as f64 - a as f64) * f)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        (lerp(r0, r1), lerp(g0, g1), lerp(b0, b1))
    }
}

/// Color-vision deficiency to correct for via daltonization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deficiency {
    Protanopia,
    Deuteranopia,
    Tritanopia,
}

impl Deficiency {
    pub fn from_name(s: &str) -> Option<Deficiency> {
        Some(match s {
            "protanopia" => Deficiency::Protanopia,
            "deuteranopia" => Deficiency::Deuteranopia,
            "tritanopia" => Deficiency::Tritanopia,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Deficiency::Protanopia => "protanopia",
            Deficiency::Deuteranopia => "deuteranopia",
            Deficiency::Tritanopia => "tritanopia",
        }
    }
}

/// Viénot/Brettel LMS-space daltonization. Simulates the deficiency, takes the
/// error versus the original, and redistributes it into channels the viewer can
/// still distinguish. Pure + deterministic.
pub fn daltonize(rgb: (u8, u8, u8), d: Deficiency) -> (u8, u8, u8) {
    let (r, g, b) = (rgb.0 as f64, rgb.1 as f64, rgb.2 as f64);

    // RGB -> LMS (Hunt-Pointer-Estevez / von Kries approximation).
    let l = 17.8824 * r + 43.5161 * g + 4.11935 * b;
    let m = 3.45565 * r + 27.1554 * g + 3.86714 * b;
    let s = 0.0299566 * r + 0.184309 * g + 1.46709 * b;

    // Simulate the deficiency by replacing one cone response.
    let (sl, sm, ss) = match d {
        Deficiency::Protanopia => (2.02344 * m - 2.52581 * s, m, s),
        Deficiency::Deuteranopia => (l, 0.494207 * l + 1.24827 * s, s),
        Deficiency::Tritanopia => (l, m, -0.395913 * l + 0.801109 * m),
    };

    // LMS -> RGB (inverse matrix) — what the deficient viewer perceives.
    // Only the red (sr) and blue (sb) perceived channels are needed: the
    // error redistribution below uses er = r - sr and eb = b - sb (the green
    // error term is unused by this canonical daltonize redistribution).
    let sr = 0.080954 * sl - 0.130504 * sm + 0.116722 * ss;
    let sb = -0.000365294 * sl - 0.00412163 * sm + 0.693511 * ss;

    // Error between the original and the perceived color.
    let er = r - sr;
    let eb = b - sb;

    // Redistribute the error into channels the viewer can still distinguish.
    let (or, og, ob) = match d {
        Deficiency::Protanopia | Deficiency::Deuteranopia => (r, g + 0.7 * er, b + 0.7 * er),
        Deficiency::Tritanopia => (r + 0.7 * eb, g + 0.7 * eb, b),
    };

    (
        or.clamp(0.0, 255.0) as u8,
        og.clamp(0.0, 255.0) as u8,
        ob.clamp(0.0, 255.0) as u8,
    )
}

/// The resolved per-frame color-assist setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorAssist {
    /// No color assist.
    #[default]
    None,
    /// Remap every pixel onto a colorblind-safe gradient by luminance.
    Remap(Palette),
    /// Daltonize existing colors for a color-vision deficiency.
    Daltonize(Deficiency),
}

impl ColorAssist {
    /// Resolve from the palette/colorblind CLI/config values. Returns `None`
    /// when neither is set (or a name is invalid). Palette takes precedence.
    pub fn from_cli(palette: Option<&str>, colorblind: Option<&str>) -> Option<ColorAssist> {
        if let Some(name) = palette {
            Palette::from_name(name).map(ColorAssist::Remap)
        } else if let Some(name) = colorblind {
            Deficiency::from_name(name).map(ColorAssist::Daltonize)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_white_is_one_black_is_zero() {
        assert!((luminance((255, 255, 255)) - 1.0).abs() < 1e-12);
        assert!(luminance((0, 0, 0)).abs() < 1e-12);
    }

    #[test]
    fn luminance_is_in_unit_interval() {
        for &(r, g, b) in &[(255, 0, 0), (0, 255, 0), (0, 0, 255), (123, 45, 200)] {
            let l = luminance((r, g, b));
            assert!(
                (0.0..=1.0).contains(&l),
                "{l} out of range for ({r},{g},{b})"
            );
        }
    }

    #[test]
    fn palette_from_name_round_trips_and_rejects_garbage() {
        use Palette::*;
        assert_eq!(Palette::from_name("viridis"), Some(Viridis));
        assert_eq!(Palette::from_name("magma"), Some(Magma));
        assert_eq!(Palette::from_name("inferno"), Some(Inferno));
        assert_eq!(Palette::from_name("plasma"), Some(Plasma));
        assert_eq!(Palette::from_name("okabe-ito"), Some(OkabeIto));
        assert_eq!(Palette::from_name("Viridis"), None); // case-sensitive
        assert_eq!(Palette::from_name("nope"), None);
    }

    #[test]
    fn palette_name_round_trips() {
        for &p in &[
            Palette::Viridis,
            Palette::Magma,
            Palette::Inferno,
            Palette::Plasma,
            Palette::OkabeIto,
        ] {
            let n = p.name();
            assert_eq!(Palette::from_name(n), Some(p));
        }
    }

    #[test]
    fn palette_endpoints_differ_and_clamp() {
        for &p in &[
            Palette::Viridis,
            Palette::Magma,
            Palette::Inferno,
            Palette::Plasma,
            Palette::OkabeIto,
        ] {
            let lo = Palette::sample(&p, 0.0);
            let hi = Palette::sample(&p, 1.0);
            assert_ne!(lo, hi, "{p:?} has no range");
            // Clamp: out-of-range t snaps to the endpoints.
            assert_eq!(Palette::sample(&p, -0.5), lo);
            assert_eq!(Palette::sample(&p, 1.5), hi);
        }
    }

    #[test]
    fn perceptual_palettes_have_monotonic_luma() {
        // viridis-family are perceptually uniform: luma rises with t.
        for &p in &[
            Palette::Viridis,
            Palette::Magma,
            Palette::Inferno,
            Palette::Plasma,
        ] {
            let mut last = -1.0;
            for i in 0..=8 {
                let t = i as f64 / 8.0;
                let l = luminance(Palette::sample(&p, t));
                assert!(l + 1e-9 >= last, "{p:?} luma not monotonic at t={t}");
                last = l;
            }
        }
    }

    #[test]
    fn deficiency_from_name_round_trips() {
        use Deficiency::*;
        assert_eq!(Deficiency::from_name("protanopia"), Some(Protanopia));
        assert_eq!(Deficiency::from_name("deuteranopia"), Some(Deuteranopia));
        assert_eq!(Deficiency::from_name("tritanopia"), Some(Tritanopia));
        assert_eq!(Deficiency::from_name("xyz"), None);
        for &d in &[Protanopia, Deuteranopia, Tritanopia] {
            assert_eq!(Deficiency::from_name(d.name()), Some(d));
        }
    }

    #[test]
    fn daltonize_stays_in_gamut() {
        let inputs = [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 255),
            (0, 0, 0),
        ];
        for &rgb in &inputs {
            for &d in &[
                Deficiency::Protanopia,
                Deficiency::Deuteranopia,
                Deficiency::Tritanopia,
            ] {
                let (r, g, b) = daltonize(rgb, d);
                assert!((0..=255).contains(&r) && (0..=255).contains(&g) && (0..=255).contains(&b));
            }
        }
    }

    #[test]
    fn daltonize_actually_shifts_affected_colors() {
        // Red is problematic for protanopia/deuteranopia; blue for tritanopia.
        // NOTE: pure (0,0,255) lies on the tritanopia neutral axis and is a
        // fixed point of the LMS redistribution, so a blue with green content
        // is used instead to exercise the transform.
        assert_ne!(
            daltonize((255, 0, 0), Deficiency::Deuteranopia),
            (255, 0, 0)
        );
        assert_ne!(daltonize((255, 0, 0), Deficiency::Protanopia), (255, 0, 0));
        assert_ne!(
            daltonize((0, 100, 255), Deficiency::Tritanopia),
            (0, 100, 255)
        );
    }

    #[test]
    fn daltonize_keeps_red_and_green_distinct_under_deuteranopia() {
        let r = daltonize((255, 0, 0), Deficiency::Deuteranopia);
        let g = daltonize((0, 255, 0), Deficiency::Deuteranopia);
        assert_ne!(r, g, "daltonization must not collapse red and green");
    }

    #[test]
    fn color_assist_from_cli_resolves() {
        use ColorAssist::*;
        // NOTE: `use ColorAssist::*` shadows `Option::None` with the
        // `ColorAssist::None` variant, so every Option-side `None` below is
        // fully qualified to disambiguate.
        assert_eq!(
            ColorAssist::from_cli(Some("viridis"), Option::None),
            Some(Remap(Palette::Viridis))
        );
        assert_eq!(
            ColorAssist::from_cli(Option::None, Some("deuteranopia")),
            Some(Daltonize(Deficiency::Deuteranopia))
        );
        assert_eq!(
            ColorAssist::from_cli(Option::None, Option::None),
            Option::None
        );
        assert_eq!(
            ColorAssist::from_cli(Some("bogus"), Option::None),
            Option::None
        );
        // If both supplied, palette wins (clap normally prevents this).
        assert_eq!(
            ColorAssist::from_cli(Some("magma"), Some("tritanopia")),
            Some(Remap(Palette::Magma))
        );
    }
}
