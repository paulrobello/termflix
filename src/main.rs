mod animations;
mod color;
mod config;
mod external;
mod gallery;
pub mod generators;
mod gif;
mod png;
mod record;
mod render;
mod rng;
// The writer thread uses raw fd + libc::write (deliberately unbuffered, to
// bypass Stdout's LineWriter). That's unix-only; on Windows main.rs writes
// frames inline via stdout.write_all() (see the cfg(not(unix)) branches).
#[cfg(unix)]
mod render_sink;

use animations::Animation;
use clap::Parser;
use crossterm::{
    cursor,
    event::{
        self, DisableFocusChange, EnableFocusChange, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute, terminal,
};
use external::{CurrentState, ExternalParams, ParamsSource, spawn_reader};
use render::{
    Canvas, ColorAssist, ColorMode, PostProcessConfig, RenderMode, pipeline::FrameEffects,
    pipeline::produce_frame, smoothing_alpha,
};
use std::io;
use std::io::IsTerminal;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "termflix", about = "Terminal animation player")]
struct Cli {
    /// Animation to play (use --list to see all)
    animation: Option<String>,

    /// Render mode (omit to use per-animation default)
    #[arg(short, long, value_enum)]
    render: Option<RenderMode>,

    /// Color mode
    #[arg(short, long, value_enum)]
    color: Option<ColorMode>,

    /// Target FPS (1-120)
    #[arg(short, long)]
    fps: Option<u32>,

    /// List available animations and exit (optional: filter by substring)
    #[arg(short, long)]
    list: Option<Option<String>>,

    /// List per-animation external parameters and exit (optional: one animation)
    #[arg(long)]
    list_params: Option<Option<String>>,

    /// Cycle through all animations (seconds per animation, 0 = disabled)
    #[arg(long)]
    cycle: Option<u32>,

    /// Record animation to .asciianim file
    #[arg(long)]
    record: Option<String>,

    /// Play back a recorded .asciianim file
    #[arg(long)]
    play: Option<String>,

    /// Export recording to GIF (requires --play)
    #[arg(long, value_name = "PATH")]
    export_gif: Option<String>,

    /// Scale factor for particle/element counts (0.5-2.0)
    #[arg(short, long)]
    scale: Option<f64>,

    /// Remove FPS cap and render as fast as possible (overrides --fps)
    #[arg(long)]
    unlimited: bool,

    /// Hide the status bar for pure animation mode
    #[arg(long)]
    clean: bool,

    /// Generate default config file at the OS config dir (see --show-config)
    #[arg(long)]
    init_config: bool,

    /// Show config file path and current settings
    #[arg(long)]
    show_config: bool,
    /// Exit on first keypress or focus when running as a screensaver
    #[arg(long)]
    screensaver: bool,

    /// Keep keybindings active in screensaver mode; any unbound key still dismisses
    #[arg(long, requires = "screensaver")]
    screensaver_keys: bool,

    /// Watch a file for external control params (ndjson — one JSON object per line)
    #[arg(long, value_name = "PATH")]
    data_file: Option<String>,

    /// Bloom/glow post-processing effect intensity (0.0-1.0)
    #[arg(long)]
    bloom_intensity: Option<f64>,

    /// Brightness threshold to trigger bloom (0.0-1.0, default 0.6)
    #[arg(long)]
    bloom_threshold: Option<f64>,

    /// Vignette edge-darkening intensity (0.0-1.0)
    #[arg(long)]
    vignette: Option<f64>,

    /// Enable CRT scanline effect
    #[arg(long)]
    scanlines: bool,

    /// Temporal brightness smoothing time constant in seconds (0 = off).
    /// Reduces flicker in fire/plasma/aurora. Toggle live with `s`.
    #[arg(long)]
    smoothing: Option<f64>,

    /// Colorblind-safe remap palette: viridis | magma | inferno | plasma | okabe-ito
    #[arg(long, conflicts_with = "colorblind")]
    palette: Option<String>,

    /// Daltonization correction type: protanopia | deuteranopia | tritanopia
    #[arg(long)]
    colorblind: Option<String>,

    /// Enable 4x4 Bayer ordered dithering in ANSI-256 mode (reduces banding).
    /// Toggle live with `d`.
    #[arg(long)]
    dither: bool,

    /// Profile per-frame timing and print summary on exit
    #[arg(long)]
    profile: bool,

    /// Disable the writer thread; write frames inline on the main thread (today's
    /// behavior). Useful for debugging and A/B comparison.
    #[arg(long)]
    single_threaded: bool,

    /// Disable dirty-cell rendering; write a full frame every time (today's behavior).
    #[arg(long)]
    full_frames: bool,

    /// Seed the RNG for deterministic, reproducible output (gallery default: 1)
    #[arg(long)]
    seed: Option<u64>,

    /// Capture animations as PNG+GIF gallery (optional: comma-separated animation names)
    #[arg(long)]
    gallery: Option<Option<String>>,

    /// Output directory for gallery captures (default: ./gallery)
    #[arg(long)]
    gallery_dir: Option<String>,

    /// Terminal width in cells for gallery captures (default: 80)
    #[arg(long)]
    gallery_cols: Option<usize>,

    /// Terminal height in cells for gallery captures (default: 25)
    #[arg(long)]
    gallery_rows: Option<usize>,

    /// Seconds of simulated time before PNG capture (default: 3.0)
    #[arg(long)]
    gallery_wait: Option<f64>,

    /// Total seconds of GIF recording for gallery captures (default: 5.0)
    #[arg(long)]
    gallery_duration: Option<f64>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // --init-config: generate default config file
    if cli.init_config {
        let path = config::config_path().expect("Could not determine config directory");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            println!("Config already exists: {}", path.display());
            println!("Delete it first if you want to regenerate.");
        } else {
            std::fs::write(&path, config::default_config_string())?;
            println!("Created config file: {}", path.display());
        }
        return Ok(());
    }

    // Load config file (defaults if not found)
    let cfg = config::load_config();
    let keybindings = build_keybindings(&cfg);

    // --show-config: display current settings
    if cli.show_config {
        let path = config::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(unknown)".to_string());
        println!("Config file: {}", path);
        println!("{:#?}", cfg);
        return Ok(());
    }

    // --gallery: capture animations to PNG+GIF
    if let Some(ref names) = cli.gallery {
        let names = names.as_ref().map(|s| {
            s.split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect::<Vec<_>>()
        });
        let config = gallery::GalleryConfig {
            dir: cli
                .gallery_dir
                .unwrap_or_else(|| "./gallery".to_string())
                .into(),
            cols: cli.gallery_cols.unwrap_or(80),
            rows: cli.gallery_rows.unwrap_or(25),
            wait_secs: cli.gallery_wait.unwrap_or(3.0),
            duration_secs: cli.gallery_duration.unwrap_or(5.0),
            names,
            seed: cli.seed.or(cfg.seed).unwrap_or(1),
        };
        return gallery::run_gallery(&config);
    }

    if let Some(ref play_path) = cli.play {
        if let Some(ref gif_path) = cli.export_gif {
            let player = record::Player::load(play_path)?;
            let frame_count = match player.frames() {
                record::Frames::V1(frames) => frames.len(),
                record::Frames::V2 { frames, .. } => frames.len(),
            };
            if frame_count == 0 {
                eprintln!("No frames to export.");
                std::process::exit(1);
            }
            let file = std::fs::File::create(gif_path)?;
            let mut writer = std::io::BufWriter::new(file);
            let result = match player.frames() {
                record::Frames::V1(frames) => {
                    let (cols, rows) = detect_recording_size(frames);
                    gif::export_gif(&mut writer, frames, cols, rows)
                }
                record::Frames::V2 { cols, rows, frames } => {
                    let pixel_frames: Vec<gif::PixelFrame> = frames
                        .iter()
                        .map(|f| gif::PixelFrame {
                            timestamp_ms: f.timestamp_ms,
                            pixels: gif::render_cells_to_pixels(&f.grid),
                        })
                        .collect();
                    gif::export_gif_pixels(&mut writer, &pixel_frames, *cols, *rows, 1)
                }
            };
            match result {
                Ok(()) => {
                    println!("Exported {} frames to {}", frame_count, gif_path);
                }
                Err(e) => {
                    eprintln!("GIF export failed: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        let player = record::Player::load(play_path)?;
        return player.play();
    }

    if let Some(target) = cli.list_params {
        let names: Vec<&str> = match target.as_deref() {
            Some(name) if animations::ANIMATION_NAMES.contains(&name) => vec![name],
            Some(name) => {
                eprintln!("Unknown animation: '{name}'\n\nAvailable animations:");
                for &(name, desc) in animations::ANIMATIONS {
                    eprintln!("  {:<12} {}", name, desc);
                }
                std::process::exit(1);
            }
            None => animations::ANIMATION_NAMES.to_vec(),
        };
        let (cols, rows) = (80usize, 24usize);
        for name in names {
            let anim = animations::create(name, cols, rows, 1.0)
                .unwrap_or_else(|| panic!("unknown animation {name}"));
            let specs = anim.param_specs();
            if specs.is_empty() {
                println!("{name}: no parameters");
                continue;
            }
            println!("{name}:");
            for s in specs {
                println!(
                    "  {:<16} {:<18} (default {}) — {}",
                    s.name,
                    format!("{}..={}", trim_f64(s.min), trim_f64(s.max)),
                    trim_f64(s.default),
                    s.help
                );
            }
        }
        println!("\nSet via external control: {{\"params\": {{\"<name>\": 0.0..1.0}}}}");
        return Ok(());
    }

    if let Some(filter) = cli.list {
        println!("Available animations:");
        let filter = filter.as_deref().map(|s| s.to_lowercase());
        let mut count = 0;
        for &(name, desc) in animations::ANIMATIONS {
            if let Some(ref f) = filter
                && !name.to_lowercase().contains(f)
                && !desc.to_lowercase().contains(f)
            {
                continue;
            }
            println!("  {:<12} {}", name, desc);
            count += 1;
        }
        if let Some(ref f) = filter {
            println!("\n  {} animation(s) matching '{}'", count, f);
        }
        println!("\nRender modes: braille, half-block, ascii");
        println!("Color modes: mono, ansi16, ansi256, true-color");
        return Ok(());
    }

    // Merge: CLI flags > config file > defaults
    let settings = resolve_settings(&cli, &cfg);

    // Validate animation name before entering raw mode so errors print cleanly
    if !animations::ANIMATION_NAMES.contains(&settings.anim_name.as_str()) {
        eprintln!(
            "Unknown animation: '{}'\n\nAvailable animations:",
            settings.anim_name
        );
        for &(name, desc) in animations::ANIMATIONS {
            eprintln!("  {:<12} {}", name, desc);
        }
        std::process::exit(1);
    }

    // Set up panic hook to restore terminal before printing panic info.
    // Without this, a panic inside raw mode leaves the terminal unusable.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = io::stdout().as_raw_fd();
            let restore = b"\x1b[?2026l\x1b[?25h\x1b[?1049l";
            unsafe {
                libc::write(fd, restore.as_ptr() as *const libc::c_void, restore.len());
            }
        }
        #[cfg(not(unix))]
        {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen);
        }
        default_hook(info);
    }));

    // Refuse terminals below the render floor before entering raw mode, so the
    // message prints on a normal cooked terminal and no restore is needed.
    let (startup_cols, startup_rows) = terminal::size()?;
    if startup_cols < MIN_TERM_COLS || startup_rows < MIN_TERM_ROWS {
        eprintln!(
            "terminal too small: termflix needs at least {MIN_TERM_COLS}x{MIN_TERM_ROWS}, got {startup_cols}x{startup_rows}"
        );
        std::process::exit(1);
    }

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
    if cli.screensaver {
        execute!(stdout, EnableFocusChange)?;
    }

    // Seeded RNG: a seeded live run is reproducible from startup. Transitions
    // between animations do not reseed, so reproducibility is start-of-run only.
    if let Some(seed) = cli.seed.or(cfg.seed) {
        rng::set_seed(seed);
    }

    let result = run_loop(settings, &keybindings);

    // Restore terminal — disable raw mode first (doesn't write to stdout)
    let _ = terminal::disable_raw_mode();

    // Flush kernel PTY buffer
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        unsafe {
            libc::tcflush(io::stdout().as_raw_fd(), libc::TCIOFLUSH);
        }
    }

    // Restore cursor and leave alt screen.
    // \x1b[?2026l MUST come first: every frame starts with \x1b[?2026h (BSU begin
    // synchronized output). If quit fires mid-write, the terminal has seen the begin
    // marker but not the end marker, so it sits in sync mode buffering everything that
    // follows — including the restore sequences — and appears frozen on the last frame.
    // Sending \x1b[?2026l closes the pending sync block; it is a no-op if not in sync mode.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = io::stdout().as_raw_fd();
        let restore = b"\x1b[?2026l\x1b[?25h\x1b[?1049l";
        unsafe {
            libc::write(fd, restore.as_ptr() as *const libc::c_void, restore.len());
        }
        // Explicitly disable focus-change reporting before exiting
        if cli.screensaver {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, DisableFocusChange);
        }
    }
    #[cfg(not(unix))]
    {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen);
        if cli.screensaver {
            let _ = execute!(stdout, DisableFocusChange);
        }
    }

    // In tmux, tell tmux to discard buffered output and force a redraw.
    // Without this, tmux slowly drains queued animation frames row by row.
    if std::env::var("TMUX").is_ok() {
        // clear-history discards tmux's output buffer for this pane
        // refresh-client forces tmux to redraw from current state
        let _ = std::process::Command::new("tmux")
            .args(["clear-history"])
            .status();
        let _ = std::process::Command::new("tmux")
            .args(["refresh-client"])
            .status();
    }

    if result.is_ok() {
        std::process::exit(0);
    }
    result
}

const RENDER_MODES: [RenderMode; 3] = [
    RenderMode::Braille,
    RenderMode::HalfBlock,
    RenderMode::Ascii,
];
const COLOR_MODES: [ColorMode; 4] = [
    ColorMode::TrueColor,
    ColorMode::Ansi256,
    ColorMode::Ansi16,
    ColorMode::Mono,
];

const TRANSITION_FRAMES: u8 = 8;

/// Minimum terminal size the renderer supports. Startup refuses to run below
/// this floor and the resize path refuses to shrink below it; animations are
/// additionally hardened to survive smaller pixel canvases (render-mode scaling
/// and gallery panes can go below the terminal floor).
const MIN_TERM_COLS: u16 = 10;
const MIN_TERM_ROWS: u16 = 5;

struct FrameProfile {
    update_us: Vec<f64>,
    render_us: Vec<f64>,
    write_us: Vec<f64>,
    total_us: Vec<f64>,
    anim_name: String,
}

impl FrameProfile {
    fn new(anim_name: &str) -> Self {
        Self {
            update_us: Vec::new(),
            render_us: Vec::new(),
            write_us: Vec::new(),
            total_us: Vec::new(),
            anim_name: anim_name.to_string(),
        }
    }

    fn record(
        &mut self,
        update_dur: Duration,
        render_dur: Duration,
        write_dur: Duration,
        total_dur: Duration,
    ) {
        self.update_us.push(update_dur.as_secs_f64() * 1e6);
        self.render_us.push(render_dur.as_secs_f64() * 1e6);
        self.write_us.push(write_dur.as_secs_f64() * 1e6);
        self.total_us.push(total_dur.as_secs_f64() * 1e6);
    }

    fn print_summary(&self) {
        if self.total_us.is_empty() {
            return;
        }
        let n = self.total_us.len();
        let stats = |data: &[f64]| -> (f64, f64, f64, f64, f64) {
            let mut sorted = data.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let sum: f64 = sorted.iter().sum();
            let p95_idx = ((n as f64) * 0.95).ceil() as usize - 1;
            (
                sum / n as f64,
                sorted[0],
                sorted[n - 1],
                sorted[p95_idx.min(n - 1)],
                sum / 1e6, // total seconds
            )
        };
        println!("\n=== Profile: {} ({} frames) ===", self.anim_name, n);
        println!(
            "{:<12} {:>10} {:>10} {:>10} {:>10}",
            "", "avg µs", "min µs", "max µs", "p95 µs"
        );
        let (avg, min, max, p95, _) = stats(&self.update_us);
        println!(
            "{:<12} {:>10.1} {:>10.1} {:>10.1} {:>10.1}",
            "update", avg, min, max, p95
        );
        let (avg, min, max, p95, _) = stats(&self.render_us);
        println!(
            "{:<12} {:>10.1} {:>10.1} {:>10.1} {:>10.1}",
            "render", avg, min, max, p95
        );
        let (avg, min, max, p95, _) = stats(&self.write_us);
        println!(
            "{:<12} {:>10.1} {:>10.1} {:>10.1} {:>10.1}",
            "write", avg, min, max, p95
        );
        let (avg, min, max, p95, total_secs) = stats(&self.total_us);
        println!(
            "{:<12} {:>10.1} {:>10.1} {:>10.1} {:>10.1}",
            "total", avg, min, max, p95
        );
        if total_secs > 0.0 {
            println!(
                "Avg FPS: {:.1} | Total time: {:.2}s",
                n as f64 / total_secs,
                total_secs
            );
        }
        println!();
    }
}

#[derive(Clone, Copy)]
enum TransitionState {
    None,
    FadingOut {
        next_anim_index: usize,
        remaining: u8,
    },
    FadingIn {
        remaining: u8,
    },
}

fn start_transition(transition: &mut TransitionState, next_anim_index: usize) {
    *transition = TransitionState::FadingOut {
        next_anim_index,
        remaining: TRANSITION_FRAMES,
    };
}

/// Spawn an animation by name. The caller has already validated the name
/// against ANIMATION_NAMES (CLI arg, arrow keys, cycle timer, external
/// control), so an unknown name here is a programming error, not user input.
fn spawn_animation(name: &str, width: usize, height: usize, scale: f64) -> Box<dyn Animation> {
    animations::create(name, width, height, scale)
        .unwrap_or_else(|| panic!("animation {name:?} not found (name should be validated first)"))
}

/// Run settings resolved once at startup from the CLI > config-file > defaults
/// merge. Passed to `run_loop` by value; run_loop owns and mutates its copy.
struct Settings {
    anim_name: String,
    render_override: Option<RenderMode>,
    color_mode: ColorMode,
    color_quant: u8,
    unlimited: bool,
    frame_dur: Duration,
    scale: f64,
    cycle: u32,
    clean: bool,
    screensaver: bool,
    screensaver_keys: bool,
    record_path: Option<String>,
    data_file: Option<String>,
    postproc: PostProcessConfig,
    smoothing_tau: f64,
    default_smoothing_tau: f64,
    default_bloom: f64,
    assist: ColorAssist,
    dither: bool,
    profile: bool,
    single_threaded: bool,
    full_frames: bool,
}

fn resolve_settings(cli: &Cli, cfg: &config::Config) -> Settings {
    let anim_name = cli
        .animation
        .clone()
        .or(cfg.animation.clone())
        .unwrap_or_else(|| "fire".to_string());
    let unlimited = cli.unlimited || cfg.unlimited_fps.unwrap_or(false);
    let fps = cli.fps.or(cfg.fps).unwrap_or(24).clamp(1, 120);
    let frame_dur = if unlimited {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(1.0 / fps as f64)
    };

    let color_mode = cli
        .color
        .or(cfg.color.map(ColorMode::from))
        .unwrap_or(ColorMode::TrueColor);
    let scale = cli.scale.or(cfg.scale).unwrap_or(1.0).clamp(0.5, 2.0);
    let cycle = cli.cycle.or(cfg.cycle).unwrap_or(0);
    let clean = cli.clean || cfg.clean.unwrap_or(false);
    let color_quant = cfg.color_quant.unwrap_or(0);
    let render_override = cli.render.or(cfg.render.map(RenderMode::from));

    let default_bloom = cli
        .bloom_intensity
        .or(cfg.postproc.and_then(|p| p.bloom))
        .unwrap_or(0.4)
        .clamp(0.0, 1.0);
    let postproc = PostProcessConfig {
        bloom: if cli.bloom_intensity.is_some() || cfg.postproc.and_then(|p| p.bloom).is_some() {
            default_bloom
        } else {
            0.0
        },
        bloom_threshold: cli
            .bloom_threshold
            .or(cfg.postproc.and_then(|p| p.bloom_threshold))
            .unwrap_or(0.6)
            .clamp(0.0, 1.0),
        vignette: cli
            .vignette
            .or(cfg.postproc.and_then(|p| p.vignette))
            .unwrap_or(0.0)
            .clamp(0.0, 1.0),
        scanlines: cli.scanlines || cfg.postproc.and_then(|p| p.scanlines).unwrap_or(false),
    };

    // Smoothing: live tau (0 = off) + the on-value the `s` key toggles to.
    let smoothing_tau = cli
        .smoothing
        .or(cfg.smoothing)
        .map(|v| v.clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let default_smoothing_tau = cli
        .smoothing
        .or(cfg.smoothing)
        .unwrap_or(0.1)
        .clamp(0.0, 1.0);

    // Colorblind-safe color assist: palette remap or daltonization (mutually exclusive).
    // CLI > config; None when unset or name invalid.
    let assist = ColorAssist::from_cli(
        cli.palette.as_deref().or(cfg.palette.as_deref()),
        cli.colorblind.as_deref().or(cfg.colorblind.as_deref()),
    )
    .unwrap_or(ColorAssist::None);
    let dither = cli.dither || cfg.dither.unwrap_or(false);

    Settings {
        anim_name,
        render_override,
        color_mode,
        color_quant,
        unlimited,
        frame_dur,
        scale,
        cycle,
        clean,
        screensaver: cli.screensaver,
        screensaver_keys: cli.screensaver_keys,
        record_path: cli.record.clone(),
        data_file: cli.data_file.clone().or(cfg.data_file.clone()),
        postproc,
        smoothing_tau,
        default_smoothing_tau,
        default_bloom,
        assist,
        dither,
        profile: cli.profile,
        single_threaded: cli.single_threaded,
        full_frames: cli.full_frames,
    }
}

/// Mutable runtime state for the main loop. Everything here changes while the
/// loop runs; startup values come from `Settings`.
struct LoopState {
    cols: u16,
    rows: u16,
    /// Status bar hidden (`clean` startup or the `h` key).
    hide_status: bool,
    /// Live dither state — the `d` key toggles this; canvas rebuilds re-apply it.
    dither: bool,
    render_mode: RenderMode,
    color_mode: ColorMode,
    scale: f64,
    postproc: PostProcessConfig,
    smoothing_tau: f64,
    /// The "on" values the `b` and `s` keys toggle back to.
    default_bloom: f64,
    default_smoothing_tau: f64,
    canvas: Canvas,
    anim: Box<dyn Animation>,
    anim_index: usize,
    transition: TransitionState,
    cycle_start: Instant,
    prev_grid: Option<render::cell::CellGrid>,
    needs_rebuild: bool,
    /// Resize cooldown — skip frames after resize.
    resize_cooldown: Instant,
    /// Adaptive frame pacing — adjusts to actual terminal throughput.
    adaptive_frame_dur: Duration,
    /// Exponential moving average of write time in secs.
    write_time_ema: f64,
    frame_count: u64,
    actual_fps: f64,
    fps_update: Instant,
    /// Merged external-control parameters (ndjson stdin/file channel).
    ext: CurrentState,
    /// Animation time accumulated with the external speed multiplier applied.
    virtual_time: f64,
}

/// Result of one keypress in the main loop.
#[derive(Debug)]
enum LoopAction {
    Continue,
    Quit,
    /// Advance to this index in ANIMATION_NAMES (arrow keys / `n` / `p`).
    Switch(usize),
}

/// Apply one keypress. Toggles mutate `state` directly; quit and animation
/// switches come back as the returned action so the loop can do its terminal
/// save/teardown bookkeeping in one place.
fn handle_key(
    state: &mut LoopState,
    code: KeyCode,
    modifiers: KeyModifiers,
    bindings: &KeyBindings,
    screensaver: bool,
    screensaver_keys: bool,
) -> LoopAction {
    // Ctrl+C always quits
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return LoopAction::Quit;
    }
    // Plain screensaver: any key dismisses.
    if screensaver && !screensaver_keys {
        return LoopAction::Quit;
    }
    match code {
        kc if bindings.quit.contains(&kc) => LoopAction::Quit,
        kc if bindings.next.contains(&kc) => {
            LoopAction::Switch((state.anim_index + 1) % animations::ANIMATION_NAMES.len())
        }
        kc if bindings.prev.contains(&kc) => LoopAction::Switch(if state.anim_index == 0 {
            animations::ANIMATION_NAMES.len() - 1
        } else {
            state.anim_index - 1
        }),
        kc if bindings.render.contains(&kc) => {
            let idx = RENDER_MODES
                .iter()
                .position(|&m| m == state.render_mode)
                .unwrap_or(0);
            state.render_mode = RENDER_MODES[(idx + 1) % RENDER_MODES.len()];
            state.needs_rebuild = true;
            LoopAction::Continue
        }
        kc if bindings.color.contains(&kc) => {
            let idx = COLOR_MODES
                .iter()
                .position(|&m| m == state.color_mode)
                .unwrap_or(0);
            state.color_mode = COLOR_MODES[(idx + 1) % COLOR_MODES.len()];
            state.needs_rebuild = true;
            LoopAction::Continue
        }
        kc if bindings.status.contains(&kc) => {
            state.hide_status = !state.hide_status;
            state.needs_rebuild = true;
            LoopAction::Continue
        }
        KeyCode::Char('b') => {
            state.postproc.bloom = if state.postproc.bloom > 0.0 {
                0.0
            } else {
                state.default_bloom
            };
            LoopAction::Continue
        }
        KeyCode::Char('s') => {
            state.smoothing_tau = if state.smoothing_tau > 0.0 {
                0.0
            } else {
                state.default_smoothing_tau
            };
            LoopAction::Continue
        }
        KeyCode::Char('d') => {
            state.dither = !state.dither;
            state.canvas.dither = state.dither;
            LoopAction::Continue
        }
        // Screensaver with keybindings active: any unbound key still dismisses.
        // (Plain screensaver already exited above; reaching here means keys are on.)
        _ if screensaver => LoopAction::Quit,
        _ => LoopAction::Continue,
    }
}

/// Rebuild canvas+animation at the current terminal size. This is the single
/// place the MIN_TERM_* floor, `color_quant` and `dither` are applied (QA-002);
/// below the floor the previous canvas is kept untouched.
fn rebuild_canvas(state: &mut LoopState, color_quant: u8) -> io::Result<()> {
    // Get the CURRENT size (may have changed since event)
    let (cur_cols, cur_rows) = terminal::size()?;
    if cur_cols >= MIN_TERM_COLS && cur_rows >= MIN_TERM_ROWS {
        state.cols = cur_cols;
        state.rows = cur_rows;
        let display_rows = if state.hide_status {
            state.rows as usize
        } else {
            (state.rows as usize).saturating_sub(1)
        };
        state.canvas = Canvas::new(
            state.cols as usize,
            display_rows,
            state.render_mode,
            state.color_mode,
        );
        state.canvas.color_quant = color_quant;
        state.canvas.dither = state.dither;
        state.anim = spawn_animation(
            animations::ANIMATION_NAMES[state.anim_index],
            state.canvas.width,
            state.canvas.height,
            state.scale,
        );
        // No clear screen — next frame overwrites everything.
        // Clearing here with a blocking flush can lock up in tmux
        // when the output buffer is full from the previous frame.
    }
    state.prev_grid = None;
    state.needs_rebuild = false;
    Ok(())
}

/// Advance the cross-fade one frame, respawning the animation when the
/// fade-out completes. Returns the intensity multiplier for this frame
/// (1.0 outside a transition).
fn step_transition(state: &mut LoopState, explicit_render: Option<RenderMode>) -> f64 {
    match state.transition {
        TransitionState::None => 1.0,
        TransitionState::FadingOut {
            next_anim_index,
            remaining,
        } => {
            let factor = remaining as f64 / TRANSITION_FRAMES as f64;
            if remaining == 0 {
                state.anim = spawn_animation(
                    animations::ANIMATION_NAMES[next_anim_index],
                    state.canvas.width,
                    state.canvas.height,
                    state.scale,
                );
                if explicit_render.is_none() {
                    state.render_mode = state.anim.preferred_render();
                    state.needs_rebuild = true;
                }
                state.prev_grid = None;
                state.transition = TransitionState::FadingIn {
                    remaining: TRANSITION_FRAMES,
                };
                0.0
            } else {
                state.transition = TransitionState::FadingOut {
                    next_anim_index,
                    remaining: remaining - 1,
                };
                factor
            }
        }
        TransitionState::FadingIn { remaining } => {
            let factor = 1.0 - remaining as f64 / TRANSITION_FRAMES as f64;
            if remaining == 0 {
                state.transition = TransitionState::None;
                1.0
            } else {
                state.transition = TransitionState::FadingIn {
                    remaining: remaining - 1,
                };
                factor
            }
        }
    }
}

/// Render the canvas to a frame string, choosing a diff against the previous
/// frame or a full redraw. Updates `prev_grid` to the frame just rendered.
/// (Recording no longer forces full frames: the recorder captures the grid
/// itself, and v2 playback re-encodes full frames from it.)
fn encode_frame(state: &mut LoopState, full_frames: bool) -> String {
    let always_reset_row_end = !matches!(state.render_mode, RenderMode::HalfBlock);
    let grid = state.canvas.render_cells();
    let frame = match &state.prev_grid {
        Some(p)
            if p.cols == grid.cols
                && p.rows == grid.rows
                && !full_frames
                && !render::encoder::grid_has_wide(&grid)
                && render::encoder::dirty_ratio(p, &grid)
                    <= render::encoder::FULL_REDRAW_THRESHOLD =>
        {
            render::encoder::encode_diff(p, &grid)
        }
        _ => render::encoder::encode_full(&grid, always_reset_row_end),
    };
    state.prev_grid = Some(grid);
    frame
}

/// Build the status bar line (reverse video, positioned at the bottom row),
/// truncated and padded to the terminal width. Empty when the bar is hidden.
fn status_line(
    state: &LoopState,
    unlimited: bool,
    recording: bool,
    assist: &ColorAssist,
) -> String {
    if state.hide_status {
        return String::new();
    }
    let rec_indicator = if recording { " [REC]" } else { "" };
    let fps_str = if unlimited {
        "∞ fps".to_string()
    } else {
        format!("{:.0} fps", state.actual_fps)
    };
    let bloom_str = if state.postproc.bloom > 0.0 {
        "ON"
    } else {
        "off"
    };
    let smooth_str = if state.smoothing_tau > 0.0 {
        "ON"
    } else {
        "off"
    };
    let dither_str = if state.dither { "ON" } else { "off" };
    let assist_str = match assist {
        ColorAssist::None => String::new(),
        ColorAssist::Remap(p) => format!(" | pal:{}", p.name()),
        ColorAssist::Daltonize(d) => format!(" | cb:{}", d.name()),
    };
    let status = format!(
        " {} | {:?} | {:?} | {}{} | bloom:{} | smooth:{} | dither:{}{assist_str} | [←/→] anim  [b] bloom  [s] smooth  [d] dither  [r] render  [c] color  [h] hide  [q] quit ",
        state.anim.name(),
        state.render_mode,
        state.color_mode,
        fps_str,
        rec_indicator,
        bloom_str,
        smooth_str,
        dither_str,
    );
    let w = state.cols as usize;
    let truncated: String = status.chars().take(w).collect();
    let padded = format!("{truncated:<width$}", width = w);
    format!("\x1b[{};1H\x1b[7m{}\x1b[0m", state.rows, padded)
}

/// Outcome of handing an assembled frame to the output path.
#[cfg(unix)]
enum FrameWrite {
    Complete,
    QuitSignaled,
    WriterDied,
}

/// Write one assembled frame buffer: through the threaded renderer when
/// available, else inline chunked writes with quit checks between chunks.
#[cfg(unix)]
fn write_frame(
    frame_buf: Vec<u8>,
    renderer: &mut Option<render_sink::ThreadedRenderer>,
    quit: &AtomicBool,
    quit_keys: &[KeyCode],
) -> io::Result<FrameWrite> {
    if let Some(r) = renderer {
        return match r.submit(frame_buf, quit, quit_keys) {
            Ok(render_sink::SubmitResult::Ok) => Ok(FrameWrite::Complete),
            Ok(render_sink::SubmitResult::Quit) => Ok(FrameWrite::QuitSignaled),
            Ok(render_sink::SubmitResult::WriterDied) => Ok(FrameWrite::WriterDied),
            Err(e) => Err(e),
        };
    }
    use std::os::unix::io::AsRawFd;
    let fd = io::stdout().as_raw_fd();
    match render_sink::write_chunked(fd, &frame_buf, || {
        if event::poll(Duration::ZERO)?
            && let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) = event::read()?
            && render_sink::is_quit_key(code, modifiers, quit_keys)
        {
            quit.store(true, Ordering::Release);
            return Ok(true);
        }
        Ok(false)
    }) {
        Ok(render_sink::WriteOutcome::Complete) => Ok(FrameWrite::Complete),
        Ok(render_sink::WriteOutcome::QuitSignaled) => Ok(FrameWrite::QuitSignaled),
        Err(e) => Err(e),
    }
}

/// What the event drain wants the main loop to do.
enum EventOutcome {
    Continue,
    Quit,
}

/// Block up to `timeout` for the next event (this doubles as the frame
/// timer), then drain every pending event. Returns `Quit` when any event
/// path requested exit; the caller performs the recording save and teardown.
fn drain_events(
    state: &mut LoopState,
    keybindings: &KeyBindings,
    screensaver: bool,
    screensaver_keys: bool,
    timeout: Duration,
) -> io::Result<EventOutcome> {
    if event::poll(timeout)? {
        // Drain all pending events
        loop {
            match event::read()? {
                Event::Resize(w, h) => {
                    state.cols = w;
                    state.rows = h;
                    state.needs_rebuild = true;
                    state.resize_cooldown = Instant::now();
                }
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) => {
                    match handle_key(
                        state,
                        code,
                        modifiers,
                        keybindings,
                        screensaver,
                        screensaver_keys,
                    ) {
                        LoopAction::Quit => return Ok(EventOutcome::Quit),
                        LoopAction::Switch(idx) => {
                            state.anim_index = idx;
                            start_transition(&mut state.transition, state.anim_index);
                            state.cycle_start = Instant::now();
                        }
                        LoopAction::Continue => {}
                    }
                }
                Event::FocusGained if screensaver && !screensaver_keys => {
                    return Ok(EventOutcome::Quit);
                }
                _ => {}
            }
            // Check for more events without blocking
            if !event::poll(Duration::ZERO)? {
                break;
            }
        }
    }
    Ok(EventOutcome::Continue)
}

/// Save the recording: leave the alt screen and drop raw mode so the save
/// message is visible, write the file, then re-enter so the exit path's own
/// restore stays symmetric.
fn save_recording(rec: record::Recorder, path: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    rec.save(path)?;
    println!("Saved {} frames to {}", rec.frame_count(), path);
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
    Ok(())
}

/// Drain the external-control channel and apply animation/scale/render/color
/// changes. Called once per frame before the animation update; render/color
/// changes flag a rebuild which the caller honors by skipping the frame.
fn apply_external_params(
    state: &mut LoopState,
    params_rx: &Option<mpsc::Receiver<ExternalParams>>,
) {
    if let Some(rx) = params_rx {
        while let Ok(p) = rx.try_recv() {
            state.ext.merge(p);
        }
    }

    // Handle animation switch from external params
    if let Some(name) = state.ext.take_animation_change()
        && animations::ANIMATION_NAMES.contains(&name.as_str())
    {
        state.anim_index = animations::ANIMATION_NAMES
            .iter()
            .position(|&n| n == name.as_str())
            .unwrap_or(state.anim_index);
        start_transition(&mut state.transition, state.anim_index);
        state.cycle_start = Instant::now();
    }

    // Handle scale change from external params
    if let Some(new_scale) = state.ext.take_scale_change() {
        state.scale = new_scale.clamp(0.5, 2.0);
        state.anim = spawn_animation(
            animations::ANIMATION_NAMES[state.anim_index],
            state.canvas.width,
            state.canvas.height,
            state.scale,
        );
        state.prev_grid = None;
    }

    // Handle render mode change from external params
    if let Some(render_name) = state.ext.take_render_change()
        && let Some(new_mode) = parse_render_mode(&render_name)
    {
        state.render_mode = new_mode;
        state.needs_rebuild = true;
    }

    // Handle color mode change from external params
    if let Some(color_name) = state.ext.take_color_change()
        && let Some(new_mode) = parse_color_mode(&color_name)
    {
        state.color_mode = new_mode;
        state.needs_rebuild = true;
    }
}

/// Roll the once-per-second fps accounting forward one frame.
fn tick_fps(state: &mut LoopState) {
    state.frame_count += 1;
    if state.fps_update.elapsed() >= Duration::from_secs(1) {
        state.actual_fps = state.frame_count as f64 / state.fps_update.elapsed().as_secs_f64();
        state.frame_count = 0;
        state.fps_update = Instant::now();
    }
}

/// The threaded renderer's own write-time measurement when active, else wall
/// time since `write_start` (the single-threaded path).
#[cfg(unix)]
fn frame_write_dur(
    renderer: Option<&render_sink::ThreadedRenderer>,
    write_start: Instant,
) -> Duration {
    match renderer {
        Some(r) => Duration::from_secs_f64(r.write_time_secs()),
        None => write_start.elapsed(),
    }
}

/// Record per-frame timings when profiling is on (write time is only known
/// after the write step).
fn record_profile(
    profile: &mut Option<FrameProfile>,
    update_dur: Duration,
    render_dur: Duration,
    write_dur: Duration,
) {
    if let Some(p) = profile {
        let total_dur = update_dur + render_dur + write_dur;
        p.record(update_dur, render_dur, write_dur, total_dur);
    }
}

/// Adaptive frame pacing: adjust frame duration based on actual write
/// throughput. In tmux, writes block when the buffer is full, so write time
/// reflects how fast tmux can actually process our output. In unlimited mode,
/// adaptive pacing prevents flooding the terminal faster than it can drain
/// (which blocks libc::write() for seconds and makes quit unresponsive); with
/// frame_dur=ZERO the target becomes write_time_ema*1.1 — no hard cap, but
/// no terminal flood either.
fn adapt_pacing(
    state: &mut LoopState,
    write_dur: Duration,
    frame_dur: Duration,
    is_tmux: bool,
    unlimited: bool,
) {
    if is_tmux || unlimited {
        state.write_time_ema = state.write_time_ema * 0.8 + write_dur.as_secs_f64() * 0.2;
        // Target: frame duration = write time + small margin for animation update
        // This ensures we never write faster than tmux can process
        let target =
            Duration::from_secs_f64((state.write_time_ema * 1.1).max(frame_dur.as_secs_f64()));
        state.adaptive_frame_dur = target.min(Duration::from_millis(200)); // cap at 5fps minimum
    }
}

fn run_loop(settings: Settings, keybindings: &KeyBindings) -> io::Result<()> {
    let Settings {
        anim_name,
        render_override: explicit_render,
        color_mode,
        color_quant,
        unlimited,
        frame_dur,
        scale,
        cycle,
        clean,
        screensaver,
        screensaver_keys,
        record_path,
        data_file,
        postproc,
        smoothing_tau,
        default_smoothing_tau,
        default_bloom,
        assist,
        dither,
        profile,
        single_threaded,
        full_frames,
    } = settings;
    // Keep the body's historical names for the two settings that changed shape.
    let initial_anim = anim_name.as_str();
    let record_path = record_path.as_deref();
    let is_tmux = std::env::var("TMUX").is_ok();

    let mut state = {
        // Placeholder pair at the minimum size, immediately replaced by the
        // rebuild below — startup goes through the same path as every resize.
        let canvas = Canvas::new(
            MIN_TERM_COLS as usize,
            MIN_TERM_ROWS as usize,
            explicit_render.unwrap_or_else(|| animations::preferred_render(initial_anim)),
            color_mode,
        );
        let anim: Box<dyn Animation> =
            spawn_animation(initial_anim, canvas.width, canvas.height, scale);
        LoopState {
            cols: MIN_TERM_COLS,
            rows: MIN_TERM_ROWS,
            hide_status: clean,
            dither,
            render_mode: explicit_render
                .unwrap_or_else(|| animations::preferred_render(initial_anim)),
            color_mode,
            scale,
            postproc,
            smoothing_tau,
            default_bloom,
            default_smoothing_tau,
            canvas,
            anim,
            anim_index: animations::ANIMATION_NAMES
                .iter()
                .position(|&n| n == initial_anim)
                .unwrap_or(0),
            transition: TransitionState::None,
            cycle_start: Instant::now(),
            prev_grid: None,
            needs_rebuild: true,
            resize_cooldown: Instant::now(),
            adaptive_frame_dur: frame_dur,
            write_time_ema: 0.0,
            frame_count: 0,
            actual_fps: 0.0,
            fps_update: Instant::now(),
            ext: CurrentState::default(),
            virtual_time: 0.0,
        }
    };
    rebuild_canvas(&mut state, color_quant)?;

    let mut last_frame = Instant::now();
    let mut recorder = record_path.map(|_| record::Recorder::new());
    // External control channel setup
    let params_rx: Option<mpsc::Receiver<ExternalParams>> = {
        if let Some(path) = data_file {
            Some(spawn_reader(ParamsSource::File(path.into())))
        } else if !std::io::stdin().is_terminal() {
            Some(spawn_reader(ParamsSource::Stdin))
        } else {
            None
        }
    };
    let mut frame_profile = profile.then(|| FrameProfile::new(initial_anim));
    let quit = Arc::new(AtomicBool::new(false));
    #[cfg(unix)]
    let mut renderer: Option<render_sink::ThreadedRenderer> = if !single_threaded {
        use std::os::unix::io::AsRawFd;
        Some(render_sink::ThreadedRenderer::new(
            quit.clone(),
            io::stdout().as_raw_fd(),
        ))
    } else {
        None
    };
    // Threaded rendering is unix-only; consume the flag on other platforms so it
    // isn't flagged as an unused parameter.
    #[cfg(not(unix))]
    let _ = single_threaded;
    let result: io::Result<()> = 'outer: loop {
        // Use event::poll as frame timer — properly yields to OS for signal handling
        let time_to_next = state
            .adaptive_frame_dur
            .saturating_sub(last_frame.elapsed());
        if matches!(
            drain_events(
                &mut state,
                keybindings,
                screensaver,
                screensaver_keys,
                time_to_next
            )?,
            EventOutcome::Quit
        ) {
            if let (Some(rec), Some(path)) = (recorder.take(), record_path) {
                save_recording(rec, path)?;
            }
            quit.store(true, Ordering::Release);
            break 'outer Ok(());
        }

        // After resize, wait for things to settle before rendering
        if state.resize_cooldown.elapsed() < Duration::from_millis(100) {
            state.needs_rebuild = true;
            continue;
        }

        // Rebuild state.canvas
        if state.needs_rebuild {
            rebuild_canvas(&mut state, color_quant)?;
            last_frame = Instant::now();
            continue; // Skip this frame, render fresh next iteration
        }

        // Auto-cycle
        if cycle > 0 && state.cycle_start.elapsed() >= Duration::from_secs(cycle as u64) {
            state.anim_index = (state.anim_index + 1) % animations::ANIMATION_NAMES.len();
            start_transition(&mut state.transition, state.anim_index);
            state.cycle_start = Instant::now();
        }

        // Timing
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f64().min(0.1); // Cap dt to avoid huge jumps
        last_frame = now;

        // Drain external params channel and apply changes
        apply_external_params(&mut state, &params_rx);

        // If a rebuild was triggered by external params, skip this frame
        if state.needs_rebuild {
            continue;
        }

        // Virtual time with speed multiplier
        let speed = state.ext.speed().clamp(0.1, 5.0);
        let effective_dt = (dt * speed).min(0.5);
        state.virtual_time += effective_dt;

        // Per-animation semantic params: the deprecated global-field overloads
        // (suppressed once named params are in use), then any pending named
        // parameters, applied once against the animation's declared specs.
        state.anim.set_params(&state.ext.legacy_overloads());
        for (name, value01) in state.ext.take_named_params() {
            let specs = state.anim.param_specs();
            if specs.iter().any(|s| s.name == name) {
                state.anim.set_param(&name, value01);
            }
        }

        // Transition fade processing. Advanced before the frame so its factor
        // feeds this frame's intensity; a mid-fade respawn at factor 0.0
        // renders black either way.
        let transition_factor = step_transition(&mut state, explicit_render);

        if state.needs_rebuild {
            continue;
        }

        // One frame through the shared pipeline (clear → update → smoothing →
        // effects → assist → post-process).
        let fx = FrameEffects {
            smoothing_alpha: (state.smoothing_tau > 0.0)
                .then(|| smoothing_alpha(effective_dt, state.smoothing_tau)),
            intensity: state.ext.intensity().clamp(0.0, 2.0) * transition_factor,
            hue_shift: state.ext.color_shift().clamp(0.0, 1.0),
            assist: &assist,
            postproc: &state.postproc,
        };
        let update_dur = produce_frame(
            state.anim.as_mut(),
            &mut state.canvas,
            effective_dt,
            state.virtual_time,
            &fx,
        );

        // Render to string
        let render_start = Instant::now();
        let frame = encode_frame(&mut state, full_frames);
        let render_dur = render_start.elapsed();

        // Record if active — capture the rendered grid (encode_frame just
        // stored it in prev_grid), not the diff-encoded string.
        if let (Some(rec), Some(grid)) = (&mut recorder, state.prev_grid.as_ref()) {
            rec.capture_grid(grid);
        }

        // Build frame buffer with synchronized output
        let mut frame_buf: Vec<u8> = Vec::with_capacity(256 * 1024);
        // Begin synchronized update — terminal batches everything until end marker
        // tmux strips these but they're harmless; direct terminals benefit from them
        frame_buf.extend_from_slice(b"\x1b[?2026h");
        frame_buf.extend_from_slice(b"\x1b[H");
        frame_buf.extend_from_slice(frame.as_bytes());

        // Status bar
        tick_fps(&mut state);
        frame_buf.extend_from_slice(
            status_line(&state, unlimited, recorder.is_some(), &assist).as_bytes(),
        );

        // Final size check — if terminal changed since we started rendering, discard frame
        let (final_cols, final_rows) = terminal::size()?;
        if final_cols != state.cols || final_rows != state.rows {
            state.cols = final_cols;
            state.rows = final_rows;
            state.needs_rebuild = true;
            state.resize_cooldown = Instant::now();
            continue; // Discard frame_buf, don't write anything
        }

        // End synchronized update
        frame_buf.extend_from_slice(b"\x1b[?2026l");

        // Write frame — on Unix, write in chunks with quit checks between each
        // so 'q' is responsive even when tmux's buffer is full.
        let write_start = Instant::now();
        #[cfg(unix)]
        match write_frame(frame_buf, &mut renderer, &quit, &keybindings.quit)? {
            FrameWrite::QuitSignaled | FrameWrite::WriterDied => break 'outer Ok(()),
            FrameWrite::Complete => {}
        }
        #[cfg(not(unix))]
        {
            use std::io::Write;
            let mut stdout = io::stdout().lock();
            stdout.write_all(&frame_buf)?;
            stdout.flush()?;
        }

        // Profile this frame and adapt pacing (write time is only known after
        // the write step).
        #[cfg(unix)]
        let write_dur = frame_write_dur(renderer.as_ref(), write_start);
        #[cfg(not(unix))]
        let write_dur = write_start.elapsed();
        record_profile(&mut frame_profile, update_dur, render_dur, write_dur);
        adapt_pacing(&mut state, write_dur, frame_dur, is_tmux, unlimited);
    };
    #[cfg(unix)]
    if let Some(r) = renderer {
        let _ = r.shutdown();
    }
    if let Some(ref p) = frame_profile {
        p.print_summary();
    }
    result
}

fn detect_recording_size(frames: &[record::Frame]) -> (usize, usize) {
    let mut max_row = 24usize;
    let mut max_col = 80usize;
    for frame in frames {
        let bytes = frame.content.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';') {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'H' {
                    let params = &frame.content.as_bytes()[start..i];
                    let s = std::str::from_utf8(params).unwrap_or("1;1");
                    let parts: Vec<&str> = s.split(';').collect();
                    if parts.len() >= 2 {
                        if let Ok(r) = parts[0].parse::<usize>() {
                            max_row = max_row.max(r.min(gif::MAX_GIF_ROWS));
                        }
                        if let Ok(c) = parts[1].parse::<usize>() {
                            max_col = max_col.max(c.min(gif::MAX_GIF_COLS));
                        }
                    }
                }
            }
            i += 1;
        }
    }
    (max_col, max_row)
}

fn parse_render_mode(s: &str) -> Option<RenderMode> {
    match s {
        "braille" => Some(RenderMode::Braille),
        "half-block" | "halfblock" => Some(RenderMode::HalfBlock),
        "ascii" => Some(RenderMode::Ascii),
        _ => None,
    }
}

/// Format a parameter bound without trailing zeros (0.05, not 0.050000…).
fn trim_f64(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

fn parse_color_mode(s: &str) -> Option<ColorMode> {
    match s {
        "mono" => Some(ColorMode::Mono),
        "ansi16" => Some(ColorMode::Ansi16),
        "ansi256" => Some(ColorMode::Ansi256),
        "true-color" | "truecolor" => Some(ColorMode::TrueColor),
        _ => None,
    }
}

fn parse_key_binding(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let s = s.trim();
    if let Some((mods, key)) = s.split_once('+') {
        let key_code = parse_key_code(key.trim())?;
        let modifiers = match mods.trim().to_ascii_lowercase().as_str() {
            "ctrl" => KeyModifiers::CONTROL,
            "alt" => KeyModifiers::ALT,
            "shift" => KeyModifiers::SHIFT,
            _ => return None,
        };
        return Some((key_code, modifiers));
    }
    let key_code = parse_key_code(s)?;
    Some((key_code, KeyModifiers::NONE))
}

fn parse_key_code(s: &str) -> Option<KeyCode> {
    match s {
        "Left" => Some(KeyCode::Left),
        "Right" => Some(KeyCode::Right),
        "Up" => Some(KeyCode::Up),
        "Down" => Some(KeyCode::Down),
        "Esc" => Some(KeyCode::Esc),
        "Enter" => Some(KeyCode::Enter),
        "Space" => Some(KeyCode::Char(' ')),
        "Tab" => Some(KeyCode::Tab),
        s if s.len() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
        _ => None,
    }
}

struct KeyBindings {
    next: Vec<KeyCode>,
    prev: Vec<KeyCode>,
    quit: Vec<KeyCode>,
    render: Vec<KeyCode>,
    color: Vec<KeyCode>,
    status: Vec<KeyCode>,
}

impl KeyBindings {
    fn defaults() -> Self {
        KeyBindings {
            next: vec![KeyCode::Right, KeyCode::Char('n')],
            prev: vec![KeyCode::Left, KeyCode::Char('p')],
            quit: vec![KeyCode::Char('q'), KeyCode::Esc],
            render: vec![KeyCode::Char('r')],
            color: vec![KeyCode::Char('c')],
            status: vec![KeyCode::Char('h')],
        }
    }
}

fn build_keybindings(cfg: &config::Config) -> KeyBindings {
    let kb = cfg.keybindings.as_ref();
    let defaults = KeyBindings::defaults();
    KeyBindings {
        next: kb
            .and_then(|m| m.get("next"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.next),
        prev: kb
            .and_then(|m| m.get("prev"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.prev),
        quit: kb
            .and_then(|m| m.get("quit"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.quit),
        render: kb
            .and_then(|m| m.get("render"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.render),
        color: kb
            .and_then(|m| m.get("color"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.color),
        status: kb
            .and_then(|m| m.get("status"))
            .and_then(|s| parse_key_binding(s))
            .map(|(c, _)| vec![c])
            .unwrap_or(defaults.status),
    }
}

#[cfg(test)]
mod detect_size_tests {
    use super::*;

    fn frame(content: &str) -> record::Frame {
        record::Frame {
            timestamp_ms: 0,
            content: content.to_string(),
        }
    }

    #[test]
    fn clamps_oversized_cursor_moves() {
        let frames = [frame("\x1b[70000;70000H")];
        assert_eq!(
            detect_recording_size(&frames),
            (gif::MAX_GIF_COLS, gif::MAX_GIF_ROWS)
        );
    }

    #[test]
    fn clamps_u32_wrap_cursor_moves() {
        let frames = [frame("\x1b[4294967296;4294967296H")];
        assert_eq!(
            detect_recording_size(&frames),
            (gif::MAX_GIF_COLS, gif::MAX_GIF_ROWS)
        );
    }

    #[test]
    fn keeps_normal_cursor_moves() {
        let frames = [frame("\x1b[10;120H")];
        assert_eq!(detect_recording_size(&frames), (120, 24));
    }

    #[test]
    fn scans_all_frames_for_max_extents() {
        let frames = [
            frame("\x1b[10;20H"),
            frame("\x1b[50;100H"),
            frame("\x1b[30;60H"),
        ];
        assert_eq!(detect_recording_size(&frames), (100, 50));
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;

    fn test_state() -> LoopState {
        let canvas = Canvas::new(20, 10, RenderMode::HalfBlock, ColorMode::TrueColor);
        LoopState {
            cols: 20,
            rows: 11,
            hide_status: false,
            dither: false,
            render_mode: RenderMode::HalfBlock,
            color_mode: ColorMode::TrueColor,
            scale: 1.0,
            postproc: PostProcessConfig::default(),
            smoothing_tau: 0.0,
            default_bloom: 0.4,
            default_smoothing_tau: 0.1,
            canvas,
            anim: spawn_animation("fire", 20, 10, 1.0),
            anim_index: 0,
            transition: TransitionState::None,
            cycle_start: Instant::now(),
            prev_grid: None,
            needs_rebuild: false,
            resize_cooldown: Instant::now(),
            adaptive_frame_dur: Duration::ZERO,
            write_time_ema: 0.0,
            frame_count: 0,
            actual_fps: 0.0,
            fps_update: Instant::now(),
            ext: CurrentState::default(),
            virtual_time: 0.0,
        }
    }

    #[test]
    fn quit_key_and_ctrl_c_return_quit() {
        let mut state = test_state();
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                &KeyBindings::defaults(),
                false,
                false
            ),
            LoopAction::Quit
        ));
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
                &KeyBindings::defaults(),
                false,
                false
            ),
            LoopAction::Quit
        ));
    }

    #[test]
    fn next_wraps_and_prev_wraps_backward() {
        let mut state = test_state();
        let kb = KeyBindings::defaults();
        // prev at index 0 wraps to the last animation
        match handle_key(
            &mut state,
            KeyCode::Left,
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        ) {
            LoopAction::Switch(idx) => {
                assert_eq!(idx, animations::ANIMATION_NAMES.len() - 1);
                state.anim_index = idx;
            }
            other => panic!("expected Switch, got {other:?}"),
        }
        // next wraps back around to 0
        match handle_key(
            &mut state,
            KeyCode::Right,
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        ) {
            LoopAction::Switch(idx) => assert_eq!(idx, 0),
            other => panic!("expected Switch, got {other:?}"),
        }
    }

    #[test]
    fn render_and_status_keys_flag_rebuild() {
        let mut state = test_state();
        let kb = KeyBindings::defaults();
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('r'),
                KeyModifiers::NONE,
                &kb,
                false,
                false
            ),
            LoopAction::Continue
        ));
        assert!(state.needs_rebuild);
        assert_ne!(state.render_mode, RenderMode::HalfBlock);

        state.needs_rebuild = false;
        handle_key(
            &mut state,
            KeyCode::Char('h'),
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        );
        assert!(state.needs_rebuild);
        assert!(state.hide_status);
    }

    #[test]
    fn toggles_flip_between_off_and_defaults() {
        let mut state = test_state();
        let kb = KeyBindings::defaults();
        // b: 0.0 -> default_bloom
        handle_key(
            &mut state,
            KeyCode::Char('b'),
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        );
        assert_eq!(state.postproc.bloom, 0.4);
        // b again: back to 0
        handle_key(
            &mut state,
            KeyCode::Char('b'),
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        );
        assert_eq!(state.postproc.bloom, 0.0);
        // s: 0.0 -> default_smoothing_tau
        handle_key(
            &mut state,
            KeyCode::Char('s'),
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        );
        assert_eq!(state.smoothing_tau, 0.1);
        // d: flips both the flag and the live canvas
        handle_key(
            &mut state,
            KeyCode::Char('d'),
            KeyModifiers::NONE,
            &kb,
            false,
            false,
        );
        assert!(state.dither);
        assert!(state.canvas.dither);
    }

    #[test]
    fn screensaver_any_key_dismisses() {
        let mut state = test_state();
        // Plain screensaver (keys disabled): unbound key quits.
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('z'),
                KeyModifiers::NONE,
                &KeyBindings::defaults(),
                true,
                false
            ),
            LoopAction::Quit
        ));
        // Screensaver with keys on: unbound key still quits, bound key acts.
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('z'),
                KeyModifiers::NONE,
                &KeyBindings::defaults(),
                true,
                true
            ),
            LoopAction::Quit
        ));
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('b'),
                KeyModifiers::NONE,
                &KeyBindings::defaults(),
                true,
                true
            ),
            LoopAction::Continue
        ));
    }

    #[test]
    fn unbound_key_is_inert_outside_screensaver() {
        let mut state = test_state();
        let before = state.clone_values();
        assert!(matches!(
            handle_key(
                &mut state,
                KeyCode::Char('z'),
                KeyModifiers::NONE,
                &KeyBindings::defaults(),
                false,
                false
            ),
            LoopAction::Continue
        ));
        assert_eq!(state.clone_values(), before);
    }

    #[test]
    fn status_line_shows_state_and_truncates_to_width() {
        let mut state = test_state();
        state.cols = 200; // wide enough that nothing is truncated
        state.actual_fps = 41.7;
        state.postproc.bloom = 0.4;
        state.smoothing_tau = 0.1;
        state.dither = true;
        let line = status_line(&state, false, false, &ColorAssist::None);
        assert!(line.starts_with("\x1b[11;1H\x1b[7m"));
        assert!(line.contains("fire"));
        assert!(line.contains("42 fps"));
        assert!(line.contains("bloom:ON"));
        assert!(line.contains("smooth:ON"));
        assert!(line.contains("dither:ON"));
        assert!(line.ends_with("\x1b[0m"));
        // Visible width (between the SGR wrappers) never exceeds cols.
        let visible = line
            .trim_start_matches("\x1b[11;1H\x1b[7m")
            .trim_end_matches("\x1b[0m");
        assert!(visible.chars().count() <= state.cols as usize);

        // Narrow terminal truncates; unlimited + recording markers show wide.
        state.cols = 10;
        let narrow = status_line(&state, true, true, &ColorAssist::None);
        let visible = narrow
            .trim_start_matches("\x1b[11;1H\x1b[7m")
            .trim_end_matches("\x1b[0m");
        assert_eq!(visible.chars().count(), 10);
        state.cols = 200;
        let wide = status_line(&state, true, true, &ColorAssist::None);
        let wide_visible = wide
            .trim_start_matches("\x1b[11;1H\x1b[7m")
            .trim_end_matches("\x1b[0m");
        assert!(wide_visible.contains("∞ fps"));
        assert!(wide_visible.contains("[REC]"));
    }

    #[test]
    fn status_line_hidden_is_empty() {
        let mut state = test_state();
        state.hide_status = true;
        assert_eq!(status_line(&state, false, false, &ColorAssist::None), "");
    }

    #[test]
    fn external_params_switch_scale_and_modes() {
        let mut state = test_state();
        state.ext.merge(ExternalParams {
            animation: Some("matrix".to_string()),
            scale: Some(1.5),
            render: Some("braille".to_string()),
            color: Some("ansi256".to_string()),
            ..Default::default()
        });
        apply_external_params(&mut state, &None);
        assert_eq!(animations::ANIMATION_NAMES[state.anim_index], "matrix");
        assert_eq!(state.scale, 1.5);
        assert_eq!(state.render_mode, RenderMode::Braille);
        assert_eq!(state.color_mode, ColorMode::Ansi256);
        assert!(state.needs_rebuild);
        assert!(matches!(
            state.transition,
            TransitionState::FadingOut { .. }
        ));
    }

    #[test]
    fn external_params_unknown_names_are_ignored() {
        let mut state = test_state();
        state.ext.merge(ExternalParams {
            animation: Some("no-such-anim".to_string()),
            render: Some("no-such-render".to_string()),
            color: Some("no-such-color".to_string()),
            ..Default::default()
        });
        apply_external_params(&mut state, &None);
        assert_eq!(state.anim_index, 0); // fire stays
        assert_eq!(state.render_mode, RenderMode::HalfBlock);
        assert_eq!(state.color_mode, ColorMode::TrueColor);
        assert!(!state.needs_rebuild);
    }

    impl LoopState {
        fn clone_values(&self) -> (bool, bool, RenderMode, ColorMode, f64, f64, bool) {
            (
                self.hide_status,
                self.dither,
                self.render_mode,
                self.color_mode,
                self.postproc.bloom,
                self.smoothing_tau,
                self.needs_rebuild,
            )
        }
    }
}
