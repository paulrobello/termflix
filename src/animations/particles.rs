use super::{Animation, ParamSpec};
use crate::generators::{ColorGradient, ColorStop, EmitterConfig, ParticleSystem};
use crate::render::Canvas;
use rand::RngExt;

/// Fireworks / particle fountain
pub struct Particles {
    width: usize,
    height: usize,
    system: ParticleSystem,
    spawn_timer: f64,
    gravity: f64,
    drag: f64,
    rng: rand::rngs::StdRng,
}

impl Particles {
    pub fn new(width: usize, height: usize, scale: f64) -> Self {
        let config = EmitterConfig {
            x: 0.0,
            y: 0.0,
            spread: std::f64::consts::TAU,
            angle: 0.0,
            speed_min: 5.0,
            speed_max: 40.0,
            life_min: 0.8,
            life_max: 2.5,
            gravity: 15.0,
            drag: 0.99,
            wind: 0.0,
            gradient: ColorGradient::new(vec![
                ColorStop {
                    t: 0.0,
                    r: 255,
                    g: 255,
                    b: 255,
                },
                ColorStop {
                    t: 1.0,
                    r: 255,
                    g: 255,
                    b: 255,
                },
            ]),
        };
        Particles {
            width,
            height,
            system: ParticleSystem::new(config, (2000.0 * scale) as usize),
            spawn_timer: 0.0,
            gravity: 15.0,
            drag: 0.99,
            rng: crate::rng::new_rng(),
        }
    }
}

impl Animation for Particles {
    fn name(&self) -> &str {
        "particles"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Braille
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn set_params(&mut self, params: &crate::external::ExternalParams) {
        // Deprecated global-field overloads; suppressed once named params are used.

        if let Some(intensity) = params.intensity {
            self.gravity = intensity.clamp(0.0, 40.0);
        }
        if let Some(cs) = params.color_shift {
            self.drag = cs.clamp(0.9, 1.0);
        }
    }

    fn param_specs(&self) -> &'static [ParamSpec] {
        &[
            ParamSpec {
                name: "gravity",
                min: 0.0,
                max: 40.0,
                default: 15.0,
                help: "downward acceleration on particles",
            },
            ParamSpec {
                name: "drag",
                min: 0.9,
                max: 1.0,
                default: 0.99,
                help: "velocity retention per frame (higher = less drag)",
            },
        ]
    }

    fn set_param(&mut self, name: &str, value01: f64) {
        let spec = self
            .param_specs()
            .iter()
            .find(|s| s.name == name)
            .expect("caller validates name against param_specs");
        let v = spec.lerp(value01);
        match name {
            "gravity" => self.gravity = v,
            "drag" => self.drag = v,
            _ => {}
        }
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, _time: f64) {
        self.spawn_timer += dt;
        if self.spawn_timer > 0.8 {
            self.spawn_timer = 0.0;
            let rng = &mut self.rng;
            let cx = rng.random_range(self.width as f64 * 0.2..self.width as f64 * 0.8);
            let cy = rng.random_range(self.height as f64 * 0.2..self.height as f64 * 0.6);
            let count = rng.random_range(30..80);
            let r: u8 = rng.random_range(100..255);
            let g: u8 = rng.random_range(100..255);
            let b: u8 = rng.random_range(100..255);

            self.system.config.x = cx;
            self.system.config.y = cy;
            self.system.emit_colored(count, (r, r), (g, g), (b, b));
        }

        self.system.config.gravity = self.gravity;
        self.system.config.drag = self.drag;
        self.system.update(dt);

        canvas.clear();
        self.system.draw_colored(canvas);
    }
}
