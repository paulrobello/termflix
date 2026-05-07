mod animations;
mod config;
mod external;
mod gallery;
pub mod generators;
mod gif;
mod png;
mod record;
mod render;

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
use render::{Canvas, ColorMode, PostProcessConfig, RenderMode};
use std::io;
use std::io::IsTerminal;
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

    /// Generate default config file at ~/.config/termflix/config.toml
    #[arg(long)]
    init_config: bool,

    /// Show config file path and current settings
    #[arg(long)]
    show_config: bool,
    /// Exit on first keypress or focus when running as a screensaver
    #[arg(long)]
    screensaver: bool,

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

    /// Profile per-frame timing and print summary on exit
    #[arg(long)]
    profile: bool,

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

    let data_file = cli.data_file.clone().or(cfg.data_file.clone());

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
        };
        return gallery::run_gallery(&config);
    }

    if let Some(ref play_path) = cli.play {
        if let Some(ref gif_path) = cli.export_gif {
            let player = record::Player::load(play_path)?;
            if player.frames().is_empty() {
                eprintln!("No frames to export.");
                std::process::exit(1);
            }
            let (cols, rows) = detect_recording_size(player.frames());
            let file = std::fs::File::create(gif_path)?;
            let mut writer = std::io::BufWriter::new(file);
            match gif::export_gif(&mut writer, player.frames(), cols, rows) {
                Ok(()) => {
                    println!("Exported {} frames to {}", player.frames().len(), gif_path);
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
    let anim_name = cli
        .animation
        .clone()
        .or(cfg.animation)
        .unwrap_or_else(|| "fire".to_string());
    let unlimited = cli.unlimited || cfg.unlimited_fps.unwrap_or(false);
    let fps = cli.fps.or(cfg.fps).unwrap_or(24).clamp(1, 120);
    let frame_dur = if unlimited {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(1.0 / fps as f64)
    };

    // Validate animation name before entering raw mode so errors print cleanly
    if !animations::ANIMATION_NAMES.contains(&anim_name.as_str()) {
        eprintln!(
            "Unknown animation: '{}'\n\nAvailable animations:",
            anim_name
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

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
    if cli.screensaver {
        execute!(stdout, EnableFocusChange)?;
    }

    // Merge remaining settings: CLI > config > defaults
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

    let result = run_loop(
        &anim_name,
        render_override,
        color_mode,
        color_quant,
        unlimited,
        frame_dur,
        scale,
        cycle,
        clean,
        cli.screensaver,
        cli.record.as_deref(),
        data_file,
        postproc,
        default_bloom,
        &keybindings,
        cli.profile,
    );

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

struct FrameProfile {
    update_us: Vec<f64>,
    render_us: Vec<f64>,
    total_us: Vec<f64>,
    anim_name: String,
}

impl FrameProfile {
    fn new(anim_name: &str) -> Self {
        Self {
            update_us: Vec::new(),
            render_us: Vec::new(),
            total_us: Vec::new(),
            anim_name: anim_name.to_string(),
        }
    }

    fn record(&mut self, update_dur: Duration, render_dur: Duration, total_dur: Duration) {
        self.update_us.push(update_dur.as_secs_f64() * 1e6);
        self.render_us.push(render_dur.as_secs_f64() * 1e6);
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

#[allow(clippy::too_many_arguments)]
fn run_loop(
    initial_anim: &str,
    explicit_render: Option<RenderMode>,
    mut color_mode: ColorMode,
    color_quant: u8,
    unlimited: bool,
    frame_dur: Duration,
    mut scale: f64,
    cycle: u32,
    clean: bool,
    screensaver: bool,
    record_path: Option<&str>,
    data_file: Option<String>,
    mut postproc: PostProcessConfig,
    default_bloom: f64,
    keybindings: &KeyBindings,
    profile: bool,
) -> io::Result<()> {
    let (mut cols, mut rows) = terminal::size()?;
    let is_tmux = std::env::var("TMUX").is_ok();
    let mut hide_status = clean;
    // Adaptive frame pacing — adjusts to actual terminal throughput
    let mut adaptive_frame_dur = frame_dur;
    let mut write_time_ema: f64 = 0.0; // exponential moving average of write time in secs

    let display_rows = if hide_status {
        rows as usize
    } else {
        (rows as usize).saturating_sub(1)
    };
    let temp_canvas = Canvas::new(
        cols as usize,
        display_rows,
        RenderMode::HalfBlock,
        color_mode,
    );
    let mut anim: Box<dyn Animation> =
        animations::create(initial_anim, temp_canvas.width, temp_canvas.height, scale)
            .expect("animation name validated before calling create");
    let mut render_mode = explicit_render.unwrap_or_else(|| anim.preferred_render());
    let mut canvas = Canvas::new(cols as usize, display_rows, render_mode, color_mode);
    canvas.color_quant = color_quant;
    anim = animations::create(initial_anim, canvas.width, canvas.height, scale)
        .expect("animation name validated before calling create");
    anim.on_resize(canvas.width, canvas.height);

    let mut anim_index = animations::ANIMATION_NAMES
        .iter()
        .position(|&n| n == initial_anim)
        .unwrap_or(0);

    let mut last_frame = Instant::now();
    let mut cycle_start = Instant::now();
    let mut frame_count: u64 = 0;
    let mut actual_fps: f64 = 0.0;
    let mut fps_update = Instant::now();
    let mut recorder = record_path.map(|_| record::Recorder::new());
    let mut needs_rebuild = false;
    // Manual frame buffer — we control when it gets written
    let mut frame_buf: Vec<u8> = Vec::with_capacity(256 * 1024);
    // Resize cooldown — skip frames after resize
    let mut resize_cooldown = Instant::now();
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
    let mut ext_state = CurrentState::default();
    let mut transition = TransitionState::None;
    let mut virtual_time: f64 = 0.0;
    let mut frame_profile = profile.then(|| FrameProfile::new(initial_anim));
    let result: io::Result<()> = 'outer: loop {
        // Use event::poll as frame timer — properly yields to OS for signal handling
        let time_to_next = adaptive_frame_dur.saturating_sub(last_frame.elapsed());
        if event::poll(time_to_next)? {
            // Drain all pending events
            loop {
                match event::read()? {
                    Event::Resize(w, h) => {
                        cols = w;
                        rows = h;
                        needs_rebuild = true;
                        resize_cooldown = Instant::now();
                    }
                    Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    }) => {
                        // Ctrl+C always quits
                        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                            break 'outer Ok(());
                        }
                        if screensaver {
                            break 'outer Ok(());
                        }
                        match code {
                            kc if keybindings.quit.contains(&kc) => {
                                if let (Some(rec), Some(path)) = (recorder.take(), record_path) {
                                    let mut stdout = io::stdout();
                                    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
                                    terminal::disable_raw_mode()?;
                                    rec.save(path)?;
                                    println!("Saved {} frames to {}", rec.frame_count(), path);
                                    terminal::enable_raw_mode()?;
                                    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
                                }
                                break 'outer Ok(());
                            }
                            kc if keybindings.next.contains(&kc) => {
                                anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
                                start_transition(&mut transition, anim_index);
                                cycle_start = Instant::now();
                            }
                            kc if keybindings.prev.contains(&kc) => {
                                anim_index = if anim_index == 0 {
                                    animations::ANIMATION_NAMES.len() - 1
                                } else {
                                    anim_index - 1
                                };
                                start_transition(&mut transition, anim_index);
                                cycle_start = Instant::now();
                            }
                            kc if keybindings.render.contains(&kc) => {
                                let idx = RENDER_MODES
                                    .iter()
                                    .position(|&m| m == render_mode)
                                    .unwrap_or(0);
                                render_mode = RENDER_MODES[(idx + 1) % RENDER_MODES.len()];
                                needs_rebuild = true;
                            }
                            kc if keybindings.color.contains(&kc) => {
                                let idx = COLOR_MODES
                                    .iter()
                                    .position(|&m| m == color_mode)
                                    .unwrap_or(0);
                                color_mode = COLOR_MODES[(idx + 1) % COLOR_MODES.len()];
                                needs_rebuild = true;
                            }
                            kc if keybindings.status.contains(&kc) => {
                                hide_status = !hide_status;
                                needs_rebuild = true;
                            }
                            KeyCode::Char('b') => {
                                postproc.bloom = if postproc.bloom > 0.0 {
                                    0.0
                                } else {
                                    default_bloom
                                };
                            }
                            _ => {}
                        }
                    }
                    Event::FocusGained if screensaver => {
                        break 'outer Ok(());
                    }
                    _ => {}
                }
                // Check for more events without blocking
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        // After resize, wait for things to settle before rendering
        if resize_cooldown.elapsed() < Duration::from_millis(100) {
            needs_rebuild = true;
            continue;
        }

        // Rebuild canvas
        if needs_rebuild {
            // Get the CURRENT size (may have changed since event)
            let (cur_cols, cur_rows) = terminal::size()?;
            if cur_cols >= 10 && cur_rows >= 5 {
                cols = cur_cols;
                rows = cur_rows;
                let display_rows = if hide_status {
                    rows as usize
                } else {
                    (rows as usize).saturating_sub(1)
                };
                canvas = Canvas::new(cols as usize, display_rows, render_mode, color_mode);
                canvas.color_quant = color_quant;
                anim = animations::create(
                    animations::ANIMATION_NAMES[anim_index],
                    canvas.width,
                    canvas.height,
                    scale,
                )
                .expect("animation name validated before calling create");
                anim.on_resize(canvas.width, canvas.height);
                // No clear screen — next frame overwrites everything.
                // Clearing here with a blocking flush can lock up in tmux
                // when the output buffer is full from the previous frame.
            }
            needs_rebuild = false;
            last_frame = Instant::now();
            continue; // Skip this frame, render fresh next iteration
        }

        // Auto-cycle
        if cycle > 0 && cycle_start.elapsed() >= Duration::from_secs(cycle as u64) {
            anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
            start_transition(&mut transition, anim_index);
            cycle_start = Instant::now();
        }

        // Timing
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f64().min(0.1); // Cap dt to avoid huge jumps
        last_frame = now;

        // Drain external params channel
        if let Some(rx) = &params_rx {
            while let Ok(p) = rx.try_recv() {
                ext_state.merge(p);
            }
        }

        // Handle animation switch from external params
        if let Some(name) = ext_state.take_animation_change()
            && animations::ANIMATION_NAMES.contains(&name.as_str())
        {
            anim_index = animations::ANIMATION_NAMES
                .iter()
                .position(|&n| n == name.as_str())
                .unwrap_or(anim_index);
            start_transition(&mut transition, anim_index);
            cycle_start = Instant::now();
        }

        // Handle scale change from external params
        if let Some(new_scale) = ext_state.take_scale_change() {
            scale = new_scale.clamp(0.5, 2.0);
            anim = animations::create(
                animations::ANIMATION_NAMES[anim_index],
                canvas.width,
                canvas.height,
                scale,
            )
            .expect("animation name validated before calling create");
            anim.on_resize(canvas.width, canvas.height);
        }

        // Handle render mode change from external params
        if let Some(render_name) = ext_state.take_render_change()
            && let Some(new_mode) = parse_render_mode(&render_name)
        {
            render_mode = new_mode;
            needs_rebuild = true;
        }

        // Handle color mode change from external params
        if let Some(color_name) = ext_state.take_color_change()
            && let Some(new_mode) = parse_color_mode(&color_name)
        {
            color_mode = new_mode;
            needs_rebuild = true;
        }

        // If a rebuild was triggered by external params, skip this frame
        if needs_rebuild {
            continue;
        }

        // Virtual time with speed multiplier
        let speed = ext_state.speed().clamp(0.1, 5.0);
        let effective_dt = (dt * speed).min(0.5);
        virtual_time += effective_dt;

        // Per-animation semantic params
        anim.set_params(ext_state.params());

        // Update animation
        let update_start = Instant::now();
        anim.update(&mut canvas, effective_dt, virtual_time);
        let update_dur = update_start.elapsed();

        // Transition fade processing
        let transition_factor = match &mut transition {
            TransitionState::None => 1.0,
            TransitionState::FadingOut {
                next_anim_index,
                remaining,
            } => {
                let factor = *remaining as f64 / TRANSITION_FRAMES as f64;
                if *remaining == 0 {
                    anim = animations::create(
                        animations::ANIMATION_NAMES[*next_anim_index],
                        canvas.width,
                        canvas.height,
                        scale,
                    )
                    .expect("animation name validated before calling create");
                    anim.on_resize(canvas.width, canvas.height);
                    if explicit_render.is_none() {
                        render_mode = anim.preferred_render();
                        needs_rebuild = true;
                    }
                    transition = TransitionState::FadingIn {
                        remaining: TRANSITION_FRAMES,
                    };
                    0.0
                } else {
                    *remaining -= 1;
                    factor
                }
            }
            TransitionState::FadingIn { remaining } => {
                let factor = 1.0 - *remaining as f64 / TRANSITION_FRAMES as f64;
                if *remaining == 0 {
                    transition = TransitionState::None;
                    1.0
                } else {
                    *remaining -= 1;
                    factor
                }
            }
        };

        if needs_rebuild {
            continue;
        }

        // Post-process canvas with intensity and hue shift
        let intensity = ext_state.intensity().clamp(0.0, 2.0) * transition_factor;
        let hue = ext_state.color_shift().clamp(0.0, 1.0);
        canvas.apply_effects(intensity, hue);
        canvas.post_process(&postproc);

        // Render to string
        let render_start = Instant::now();
        let frame = canvas.render();
        let render_dur = render_start.elapsed();

        // Profile this frame
        if let Some(ref mut p) = frame_profile {
            let total_dur = update_dur + render_dur;
            p.record(update_dur, render_dur, total_dur);
        }

        // Record if active
        if let Some(ref mut rec) = recorder {
            rec.capture(&frame);
        }

        // Build frame buffer with synchronized output
        frame_buf.clear();
        // Begin synchronized update — terminal batches everything until end marker
        // tmux strips these but they're harmless; direct terminals benefit from them
        frame_buf.extend_from_slice(b"\x1b[?2026h");
        frame_buf.extend_from_slice(b"\x1b[H");
        frame_buf.extend_from_slice(frame.as_bytes());

        // Status bar
        frame_count += 1;
        if fps_update.elapsed() >= Duration::from_secs(1) {
            actual_fps = frame_count as f64 / fps_update.elapsed().as_secs_f64();
            frame_count = 0;
            fps_update = Instant::now();
        }
        if !hide_status {
            let rec_indicator = if recorder.is_some() { " [REC]" } else { "" };
            let fps_str = if unlimited {
                "∞ fps".to_string()
            } else {
                format!("{:.0} fps", actual_fps)
            };
            let bloom_str = if postproc.bloom > 0.0 { "ON" } else { "off" };
            let status = format!(
                " {} | {:?} | {:?} | {}{} | bloom:{} | [←/→] anim  [b] bloom  [r] render  [c] color  [h] hide  [q] quit ",
                anim.name(),
                render_mode,
                color_mode,
                fps_str,
                rec_indicator,
                bloom_str,
            );
            let w = cols as usize;
            let truncated: String = status.chars().take(w).collect();
            let padded = format!("{:<width$}", truncated, width = w);
            frame_buf
                .extend_from_slice(format!("\x1b[{};1H\x1b[7m{}\x1b[0m", rows, padded).as_bytes());
        }

        // Final size check — if terminal changed since we started rendering, discard frame
        let (final_cols, final_rows) = terminal::size()?;
        if final_cols != cols || final_rows != rows {
            cols = final_cols;
            rows = final_rows;
            needs_rebuild = true;
            resize_cooldown = Instant::now();
            continue; // Discard frame_buf, don't write anything
        }

        // End synchronized update
        frame_buf.extend_from_slice(b"\x1b[?2026l");

        // Write frame — on Unix, write in chunks with quit checks between each
        // so 'q' is responsive even when tmux's buffer is full.
        let write_start = Instant::now();
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = io::stdout().as_raw_fd();
            let mut written = 0;
            let buf = &frame_buf;
            while written < buf.len() {
                if event::poll(Duration::ZERO)?
                    && let Event::Key(KeyEvent {
                        code,
                        kind: KeyEventKind::Press,
                        modifiers,
                        ..
                    }) = event::read()?
                    && (keybindings.quit.contains(&code)
                        || (code == KeyCode::Char('c')
                            && modifiers.contains(KeyModifiers::CONTROL)))
                {
                    break 'outer Ok(());
                }
                let chunk_end = (written + 16384).min(buf.len());
                let n = unsafe {
                    libc::write(
                        fd,
                        buf[written..chunk_end].as_ptr() as *const libc::c_void,
                        chunk_end - written,
                    )
                };
                if n > 0 {
                    written += n as usize;
                    // Also check for quit AFTER the write — catches events that arrived
                    // during a blocking write (e.g. during EMA warmup in unlimited mode).
                    if event::poll(Duration::ZERO)?
                        && let Event::Key(KeyEvent {
                            code,
                            kind: KeyEventKind::Press,
                            modifiers,
                            ..
                        }) = event::read()?
                        && (keybindings.quit.contains(&code)
                            || (code == KeyCode::Char('c')
                                && modifiers.contains(KeyModifiers::CONTROL)))
                    {
                        break 'outer Ok(());
                    }
                } else if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    break 'outer Err(err);
                }
            }
        }
        #[cfg(not(unix))]
        {
            use std::io::Write;
            let mut stdout = io::stdout().lock();
            stdout.write_all(&frame_buf)?;
            stdout.flush()?;
        }

        // Adaptive frame pacing: adjust frame duration based on actual write throughput.
        // In tmux, writes block when the buffer is full, so write time reflects
        // how fast tmux can actually process our output.
        // In unlimited mode, also enable adaptive pacing: without it, we flood the
        // terminal faster than it can drain, causing libc::write() to block for seconds
        // and making quit unresponsive. With frame_dur=ZERO, the target becomes
        // write_time_ema*1.1 — no hard cap, but no terminal flood either.
        if is_tmux || unlimited {
            let write_secs = write_start.elapsed().as_secs_f64();
            write_time_ema = write_time_ema * 0.8 + write_secs * 0.2;
            // Target: frame duration = write time + small margin for animation update
            // This ensures we never write faster than tmux can process
            let target =
                Duration::from_secs_f64((write_time_ema * 1.1).max(frame_dur.as_secs_f64()));
            adaptive_frame_dur = target.min(Duration::from_millis(200)); // cap at 5fps minimum
        }
    };
    if let Some(ref p) = frame_profile {
        p.print_summary();
    }
    result
}

fn detect_recording_size(frames: &[record::Frame]) -> (usize, usize) {
    let mut max_row = 24usize;
    let mut max_col = 80usize;
    if let Some(frame) = frames.first() {
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
                            max_row = max_row.max(r);
                        }
                        if let Ok(c) = parts[1].parse::<usize>() {
                            max_col = max_col.max(c);
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
