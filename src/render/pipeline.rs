//! The one frame pipeline shared by the live loop, the gallery and the
//! encoder benchmarks: clear → update → smoothing → effects → color assist →
//! post-process. Everything that touches a finished frame goes through
//! [`produce_frame`] so gallery output matches live output and a new
//! post-effect is added in exactly one place.

use std::time::Duration;

use crate::animations::Animation;
use crate::render::canvas::{Canvas, PostProcessConfig};
use crate::render::color_assist::ColorAssist;

/// Per-frame effect settings applied after the animation draws.
pub struct FrameEffects<'a> {
    /// Temporal smoothing alpha; `None` disables smoothing.
    pub smoothing_alpha: Option<f64>,
    /// Intensity multiplier, including any transition fade factor.
    pub intensity: f64,
    /// Hue shift in the 0..=1 range.
    pub hue_shift: f64,
    /// Colorblind-safe assist applied after effects.
    pub assist: &'a ColorAssist,
    /// Bloom / vignette / scanlines.
    pub postproc: &'a PostProcessConfig,
}

impl FrameEffects<'_> {
    /// Neutral settings: no smoothing, no assist, no post-processing — the
    /// frame is exactly what the animation drew.
    pub fn neutral<'a>(
        assist: &'a ColorAssist,
        postproc: &'a PostProcessConfig,
    ) -> FrameEffects<'a> {
        FrameEffects {
            smoothing_alpha: None,
            intensity: 1.0,
            hue_shift: 0.0,
            assist,
            postproc,
        }
    }
}

/// Clear, update, and post-process one frame. Returns time spent in `update`.
///
/// The canvas arrives cleared for the animation; `apply_smoothing` blends
/// against the separate `prev_pixels` buffer, so clearing first does not
/// disturb smoothing.
pub fn produce_frame(
    anim: &mut dyn Animation,
    canvas: &mut Canvas,
    dt: f64,
    t: f64,
    fx: &FrameEffects,
) -> Duration {
    canvas.clear();
    let update_start = std::time::Instant::now();
    anim.update(canvas, dt, t);
    let update_dur = update_start.elapsed();
    if let Some(alpha) = fx.smoothing_alpha {
        canvas.apply_smoothing(alpha);
    }
    canvas.apply_effects(fx.intensity, fx.hue_shift);
    canvas.apply_color_assist(fx.assist);
    canvas.post_process(fx.postproc);
    update_dur
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::canvas::{ColorMode, RenderMode};

    /// Stub that paints one deterministic pixel and never clears, so the test
    /// proves the pipeline (not the animation) owns clearing.
    struct OnePixel;
    impl Animation for OnePixel {
        fn name(&self) -> &str {
            "one_pixel"
        }
        fn update(&mut self, canvas: &mut Canvas, _dt: f64, _t: f64) {
            canvas.set_colored(3, 2, 0.8, 255, 64, 32);
        }
    }

    #[test]
    fn produce_frame_matches_hand_rolled_live_sequence() {
        let assist = ColorAssist::None;
        let postproc = PostProcessConfig {
            bloom: 0.4,
            bloom_threshold: 0.6,
            vignette: 0.2,
            scanlines: false,
        };
        let alpha = 0.25;

        let mut pipeline_canvas = Canvas::new(20, 10, RenderMode::HalfBlock, ColorMode::TrueColor);
        let mut hand_canvas = Canvas::new(20, 10, RenderMode::HalfBlock, ColorMode::TrueColor);
        // Warm both smoothing histories the same way so the alpha blend sees
        // identical prev_pixels.
        let mut a = OnePixel;
        let mut b = OnePixel;
        for canvas in [&mut pipeline_canvas, &mut hand_canvas] {
            a.update(canvas, 0.0, 0.0);
            canvas.apply_smoothing(alpha);
        }

        let fx = FrameEffects {
            smoothing_alpha: Some(alpha),
            intensity: 1.5,
            hue_shift: 0.1,
            assist: &assist,
            postproc: &postproc,
        };
        produce_frame(&mut a, &mut pipeline_canvas, 1.0 / 24.0, 1.0, &fx);

        // The hand-rolled live sequence (pre-ENH-002 order).
        hand_canvas.clear();
        b.update(&mut hand_canvas, 1.0 / 24.0, 1.0);
        hand_canvas.apply_smoothing(alpha);
        hand_canvas.apply_effects(1.5, 0.1);
        hand_canvas.apply_color_assist(&assist);
        hand_canvas.post_process(&postproc);

        assert_eq!(pipeline_canvas.pixels, hand_canvas.pixels);
        assert_eq!(pipeline_canvas.colors, hand_canvas.colors);
    }

    #[test]
    fn produce_frame_clears_before_update() {
        let assist = ColorAssist::None;
        let postproc = PostProcessConfig::default();
        let mut canvas = Canvas::new(20, 10, RenderMode::HalfBlock, ColorMode::TrueColor);
        // Dirty the canvas; a non-clearing animation must still see it blank.
        canvas.set_colored(0, 0, 1.0, 255, 255, 255);
        produce_frame(
            &mut OnePixel,
            &mut canvas,
            0.0,
            0.0,
            &FrameEffects::neutral(&assist, &postproc),
        );
        assert_eq!(canvas.pixels[0], 0.0);
    }
}
