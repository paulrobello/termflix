pub mod atom;
pub mod aurora;
pub mod automata;
pub mod blackhole;
pub mod boids;
pub mod campfire;
pub mod cells;
pub mod crystallize;
pub mod dna;
pub mod dragon;
pub mod eclipse;
pub mod fire;
pub mod fireflies;
pub mod flappy_bird;
pub mod flow_field;
pub mod fountain;
pub mod garden;
pub mod globe;
pub mod hackerman;
pub mod invaders;
pub mod langton;
pub mod lava;
pub mod life;
pub mod lightning;
pub mod mandelbrot;
pub mod matrix;
pub mod maze;
pub mod metaballs;
pub mod nbody;
pub mod ocean;
pub mod particles;
pub mod pendulum;
pub mod petals;
pub mod plasma;
pub mod pong;
pub mod pulse;
pub mod radar;
pub mod rain;
pub mod rainforest;
pub mod reaction_diffusion;
pub mod ripple;
pub mod sandstorm;
pub mod sierpinski;
pub mod smoke;
pub mod snake;
pub mod snow;
pub mod sort;
pub mod spiral;
pub mod starfield;
pub mod tetris;
pub mod visualizer;
pub mod voronoi;
pub mod waterfall;
pub mod wave;

use crate::render::{Canvas, RenderMode};

/// Every animation implements this trait
pub trait Animation {
    /// Human-readable name
    fn name(&self) -> &str;

    /// Advance the simulation and draw into the canvas
    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64);

    /// Preferred render mode for this animation (used when no -r flag given)
    fn preferred_render(&self) -> RenderMode {
        RenderMode::HalfBlock
    }

    /// Called once per frame before update(). Default is a no-op.
    fn set_params(&mut self, _params: &crate::external::ExternalParams) {}

    /// Called when the canvas is rebuilt with new dimensions.
    /// Override to update stored dimensions and rebuild size-dependent state.
    fn on_resize(&mut self, _width: usize, _height: usize) {}

    /// Returns a list of supported external control parameters.
    /// Each entry is `(param_name, min_value, max_value)`.
    /// The empty slice default means the animation has no tunable parameters.
    #[allow(dead_code)]
    fn supported_params(&self) -> &'static [(&'static str, f64, f64)] {
        &[]
    }
}

macro_rules! declare_animations {
    ($(( $name:literal, $path:path, $desc:literal )),* $(,)?) => {
        pub const ANIMATIONS: &[(&str, &str)] = &[
            $( ($name, $desc), )*
        ];

        pub const ANIMATION_NAMES: &[&str] = &[
            $( $name, )*
        ];

        pub fn create(name: &str, width: usize, height: usize, scale: f64) -> Option<Box<dyn Animation>> {
            Some(match name {
                $( $name => Box::new(<$path>::new(width, height, scale)), )*
                _ => return None,
            })
        }
    }
}

declare_animations! {
    ("fire", fire::Fire, "Doom-style fire effect with heat propagation"),
    ("matrix", matrix::Matrix, "Matrix digital rain with trailing drops"),
    ("plasma", plasma::Plasma, "Classic plasma with overlapping sine waves"),
    ("starfield", starfield::Starfield, "3D starfield with depth parallax"),
    ("wave", wave::Wave, "Sine wave interference from moving sources"),
    ("life", life::GameOfLife, "Conway's Game of Life cellular automaton"),
    ("particles", particles::Particles, "Fireworks bursting with physics and fade"),
    ("pendulum", pendulum::Pendulum, "Pendulum wave with mesmerizing phase patterns"),
    ("rain", rain::Rain, "Raindrops with splash particles and wind"),
    ("fountain", fountain::Fountain, "Water fountain with jets, splashes, and mist"),
    ("flow", flow_field::FlowField, "Perlin noise flow field with particle trails"),
    ("spiral", spiral::Spiral, "Rotating multi-arm spiral pattern"),
    ("ocean", ocean::Ocean, "Ocean waves with foam and depth shading"),
    ("aurora", aurora::Aurora, "Aurora borealis with layered curtains"),
    ("lightning", lightning::Lightning, "Lightning bolts with recursive branching"),
    ("smoke", smoke::Smoke, "Smoke rising with Perlin turbulence"),
    ("ripple", ripple::Ripple, "Ripple interference from random drop points"),
    ("snow", snow::Snow, "Snowfall with accumulation on the ground"),
    ("garden", garden::Garden, "Growing garden with rain, clouds, and blooming plants"),
    ("fireflies", fireflies::Fireflies, "Fireflies blinking with warm glow"),
    ("dna", dna::Dna, "Rotating DNA double helix with base pairs"),
    ("pulse", pulse::Pulse, "Expanding pulse rings from center"),
    ("boids", boids::Boids, "Boids flocking simulation with trails"),
    ("lava", lava::Lava, "Lava lamp blobs rising, merging, and splitting"),
    ("sandstorm", sandstorm::Sandstorm, "Blowing sand with dune formation"),
    ("petals", petals::Petals, "Cherry blossom petals drifting in wind"),
    ("campfire", campfire::Campfire, "Campfire with rising ember sparks"),
    ("waterfall", waterfall::Waterfall, "Cascading water with mist spray"),
    ("eclipse", eclipse::Eclipse, "Moon crossing sun with corona rays"),
    ("blackhole", blackhole::Blackhole, "Black hole with accretion disk and lensing"),
    ("radar", radar::Radar, "Rotating radar sweep with fading blips"),
    ("rainforest", rainforest::Rainforest, "Layered rainforest with parallax scrolling, rain, birds, and falling leaves"),
    ("crystallize", crystallize::Crystallize, "DLA crystal growth from center seed"),
    ("hackerman", hackerman::Hackerman, "Scrolling hex/binary hacker terminal"),
    ("visualizer", visualizer::Visualizer, "Audio spectrum analyzer with bouncing bars"),
    ("cells", cells::Cells, "Cell division and mitosis animation"),
    ("atom", atom::Atom, "Electrons orbiting a nucleus in 3D"),
    ("automata", automata::Automata, "Cellular automata cycling through multiple rulesets"),
    ("globe", globe::Globe, "Rotating wireframe Earth with continents"),
    ("dragon", dragon::Dragon, "Dragon curve fractal with color cycling"),
    ("sierpinski", sierpinski::Sierpinski, "Animated Sierpinski triangle with zoom"),
    ("mandelbrot", mandelbrot::Mandelbrot, "Mandelbrot set with zoom and color cycling"),
    ("maze", maze::Maze, "Animated maze generation with recursive backtracking and BFS solving"),
    ("metaballs", metaballs::Metaballs, "Organic metaballs merging and splitting with smooth distance fields"),
    ("nbody", nbody::NBody, "N-body gravitational simulation with colorful orbiting masses and merging"),
    ("langton", langton::Langton, "Langton's Ant cellular automaton"),
    ("sort", sort::Sort, "Sorting algorithm visualizer"),
    ("tetris", tetris::Tetris, "Self-playing Tetris with AI piece placement"),
    ("snake", snake::Snake, "Self-playing Snake game AI"),
    ("invaders", invaders::Invaders, "Space Invaders attract mode demo"),
    ("pong", pong::Pong, "Self-playing Pong with AI paddles"),
    ("flappy_bird", flappy_bird::FlappyBird, "Self-playing Flappy Bird with AI"),
    ("reaction_diffusion", reaction_diffusion::ReactionDiffusion, "Gray-Scott reaction-diffusion coral/brain patterns"),
    ("voronoi", voronoi::Voronoi, "Animated Voronoi diagram with drifting colored cells and edge detection"),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_returns_some_for_all_known_names() {
        for &name in ANIMATION_NAMES {
            let result = create(name, 80, 24, 1.0);
            assert!(result.is_some(), "create({name:?}) returned None");
        }
    }

    #[test]
    fn test_create_returns_none_for_unknown_name() {
        let result = create("does_not_exist", 80, 24, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_animation_names_and_animations_have_same_length() {
        assert_eq!(ANIMATION_NAMES.len(), ANIMATIONS.len());
    }

    #[test]
    fn test_animation_names_match_animations_list() {
        for (name, (anim_name, _desc)) in ANIMATION_NAMES.iter().zip(ANIMATIONS.iter()) {
            assert_eq!(
                name, anim_name,
                "ANIMATION_NAMES and ANIMATIONS are out of sync at {name}"
            );
        }
    }

    #[test]
    fn test_created_animation_name_matches_requested() {
        let anim = create("fire", 80, 24, 1.0).unwrap();
        assert_eq!(anim.name(), "fire");
    }

    #[test]
    fn test_fire_supported_params_includes_intensity() {
        let anim = create("fire", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty(), "fire should have supported params");
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
    }

    #[test]
    fn test_plasma_supported_params_includes_color_shift() {
        let anim = create("plasma", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty(), "plasma should have supported params");
        assert!(params.iter().any(|&(name, _, _)| name == "color_shift"));
    }

    #[test]
    fn test_unknown_animation_has_empty_params() {
        // Most animations have no declared params — verify default returns empty
        let anim = create("matrix", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(params.is_empty(), "matrix should have no declared params");
    }

    #[test]
    fn test_boids_supported_params() {
        let anim = create("boids", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
        assert!(params.iter().any(|&(name, _, _)| name == "color_shift"));
    }

    #[test]
    fn test_particles_supported_params() {
        let anim = create("particles", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
    }

    #[test]
    fn test_wave_supported_params() {
        let anim = create("wave", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "intensity"));
        assert!(params.iter().any(|&(name, _, _)| name == "color_shift"));
    }

    #[test]
    fn test_sort_supported_params() {
        let anim = create("sort", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "speed"));
    }

    #[test]
    fn test_snake_supported_params() {
        let anim = create("snake", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "speed"));
    }

    #[test]
    fn test_pong_supported_params() {
        let anim = create("pong", 80, 24, 1.0).unwrap();
        let params = anim.supported_params();
        assert!(!params.is_empty());
        assert!(params.iter().any(|&(name, _, _)| name == "speed"));
    }
}
