pub mod atom;
pub mod aurora;
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
pub mod flow_field;
pub mod fountain;
pub mod globe;
pub mod hackerman;
pub mod invaders;
pub mod langton;
pub mod lava;
pub mod life;
pub mod lightning;
pub mod mandelbrot;
pub mod matrix;
pub mod ocean;
pub mod particles;
pub mod petals;
pub mod plasma;
pub mod pong;
pub mod pulse;
pub mod radar;
pub mod rain;
pub mod ripple;
pub mod sandstorm;
pub mod sierpinski;
pub mod smoke;
pub mod snake;
pub mod snow;
pub mod sort;
pub mod spiral;
pub mod starfield;
pub mod visualizer;
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
}

/// List of all available animation names with descriptions.
pub const ANIMATIONS: &[(&str, &str)] = &[
    ("fire", "Doom-style fire effect with heat propagation"),
    ("matrix", "Matrix digital rain with trailing drops"),
    ("plasma", "Classic plasma with overlapping sine waves"),
    ("starfield", "3D starfield with depth parallax"),
    ("wave", "Sine wave interference from moving sources"),
    ("life", "Conway's Game of Life cellular automaton"),
    ("particles", "Fireworks bursting with physics and fade"),
    ("rain", "Raindrops with splash particles and wind"),
    ("fountain", "Water fountain with jets, splashes, and mist"),
    ("flow", "Perlin noise flow field with particle trails"),
    ("spiral", "Rotating multi-arm spiral pattern"),
    ("ocean", "Ocean waves with foam and depth shading"),
    ("aurora", "Aurora borealis with layered curtains"),
    ("lightning", "Lightning bolts with recursive branching"),
    ("smoke", "Smoke rising with Perlin turbulence"),
    ("ripple", "Ripple interference from random drop points"),
    ("snow", "Snowfall with accumulation on the ground"),
    ("fireflies", "Fireflies blinking with warm glow"),
    ("dna", "Rotating DNA double helix with base pairs"),
    ("pulse", "Expanding pulse rings from center"),
    ("boids", "Boids flocking simulation with trails"),
    ("lava", "Lava lamp blobs rising, merging, and splitting"),
    ("sandstorm", "Blowing sand with dune formation"),
    ("petals", "Cherry blossom petals drifting in wind"),
    ("campfire", "Campfire with rising ember sparks"),
    ("waterfall", "Cascading water with mist spray"),
    ("eclipse", "Moon crossing sun with corona rays"),
    ("blackhole", "Black hole with accretion disk and lensing"),
    ("radar", "Rotating radar sweep with fading blips"),
    ("crystallize", "DLA crystal growth from center seed"),
    ("hackerman", "Scrolling hex/binary hacker terminal"),
    ("visualizer", "Audio spectrum analyzer with bouncing bars"),
    ("cells", "Cell division and mitosis animation"),
    ("atom", "Electrons orbiting a nucleus in 3D"),
    ("globe", "Rotating wireframe Earth with continents"),
    ("dragon", "Dragon curve fractal with color cycling"),
    ("sierpinski", "Animated Sierpinski triangle with zoom"),
    ("mandelbrot", "Mandelbrot set with zoom and color cycling"),
    ("langton", "Langton's Ant cellular automaton"),
    ("sort", "Sorting algorithm visualizer"),
    ("snake", "Self-playing Snake game AI"),
    ("invaders", "Space Invaders attract mode demo"),
    ("pong", "Self-playing Pong with AI paddles"),
];

/// List of all available animation names.
pub const ANIMATION_NAMES: &[&str] = &[
    "fire",
    "matrix",
    "plasma",
    "starfield",
    "wave",
    "life",
    "particles",
    "rain",
    "fountain",
    "flow",
    "spiral",
    "ocean",
    "aurora",
    "lightning",
    "smoke",
    "ripple",
    "snow",
    "fireflies",
    "dna",
    "pulse",
    "boids",
    "lava",
    "sandstorm",
    "petals",
    "campfire",
    "waterfall",
    "eclipse",
    "blackhole",
    "radar",
    "crystallize",
    "hackerman",
    "visualizer",
    "cells",
    "atom",
    "globe",
    "dragon",
    "sierpinski",
    "mandelbrot",
    "langton",
    "sort",
    "snake",
    "invaders",
    "pong",
];

/// Create an animation by name with scale factor for particle/element counts.
pub fn create(name: &str, width: usize, height: usize, scale: f64) -> Box<dyn Animation> {
    match name {
        "fire" => Box::new(fire::Fire::new(width, height)),
        "matrix" => Box::new(matrix::Matrix::new(width, height, scale)),
        "plasma" => Box::new(plasma::Plasma::new()),
        "starfield" => Box::new(starfield::Starfield::new(width, height, scale)),
        "wave" => Box::new(wave::Wave::new()),
        "life" => Box::new(life::GameOfLife::new(width, height)),
        "particles" => Box::new(particles::Particles::new(width, height, scale)),
        "rain" => Box::new(rain::Rain::new(width, height, scale)),
        "fountain" => Box::new(fountain::Fountain::new(width, height, scale)),
        "flow" => Box::new(flow_field::FlowField::new(width, height, scale)),
        "spiral" => Box::new(spiral::Spiral::new()),
        "ocean" => Box::new(ocean::Ocean::new()),
        "aurora" => Box::new(aurora::Aurora::new()),
        "lightning" => Box::new(lightning::Lightning::new(width, height)),
        "smoke" => Box::new(smoke::Smoke::new(width, height, scale)),
        "ripple" => Box::new(ripple::Ripple::new(width, height)),
        "snow" => Box::new(snow::Snow::new(width, height, scale)),
        "fireflies" => Box::new(fireflies::Fireflies::new(width, height, scale)),
        "dna" => Box::new(dna::Dna::new()),
        "pulse" => Box::new(pulse::Pulse::new(width, height)),
        "boids" => Box::new(boids::Boids::new(width, height, scale)),
        "lava" => Box::new(lava::Lava::new(width, height, scale)),
        "sandstorm" => Box::new(sandstorm::Sandstorm::new(width, height, scale)),
        "petals" => Box::new(petals::Petals::new(width, height, scale)),
        "campfire" => Box::new(campfire::Campfire::new(width, height, scale)),
        "waterfall" => Box::new(waterfall::Waterfall::new(width, height, scale)),
        "eclipse" => Box::new(eclipse::Eclipse::new()),
        "blackhole" => Box::new(blackhole::Blackhole::new()),
        "radar" => Box::new(radar::Radar::new()),
        "crystallize" => Box::new(crystallize::Crystallize::new(width, height, scale)),
        "hackerman" => Box::new(hackerman::Hackerman::new(width, height, scale)),
        "visualizer" => Box::new(visualizer::Visualizer::new(width, height, scale)),
        "cells" => Box::new(cells::Cells::new(width, height, scale)),
        "atom" => Box::new(atom::Atom::new()),
        "globe" => Box::new(globe::Globe::new()),
        "dragon" => Box::new(dragon::Dragon::new()),
        "sierpinski" => Box::new(sierpinski::Sierpinski::new()),
        "mandelbrot" => Box::new(mandelbrot::Mandelbrot::new()),
        "langton" => Box::new(langton::Langton::new(width, height, scale)),
        "sort" => Box::new(sort::Sort::new(width, height, scale)),
        "snake" => Box::new(snake::Snake::new(width, height, scale)),
        "invaders" => Box::new(invaders::Invaders::new(width, height, scale)),
        "pong" => Box::new(pong::Pong::new(width, height, scale)),
        _ => panic!("Unknown animation: {}", name),
    }
}
