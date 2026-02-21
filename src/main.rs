mod animations;
pub mod generators;
mod record;
mod render;

use std::io::{self, Write};
use std::time::{Duration, Instant};
use crossterm::{
    cursor, execute, terminal,
    event::{self, Event, KeyCode, KeyEvent},
};
use clap::Parser;
use render::{Canvas, ColorMode, RenderMode};
use animations::Animation;

#[derive(Parser)]
#[command(name = "termflix", about = "Terminal animation player")]
struct Cli {
    /// Animation to play
    /// Name of animation (use --list to see all)
    animation: Option<String>,

    /// Render mode (omit to use per-animation default)
    #[arg(short, long, value_enum)]
    render: Option<RenderMode>,

    /// Color mode
    #[arg(short, long, value_enum, default_value = "true-color")]
    color: ColorMode,

    /// Target FPS (1-120)
    #[arg(short, long, default_value = "24")]
    fps: u32,

    /// List available animations and exit
    #[arg(short, long)]
    list: bool,

    /// Cycle through all animations (seconds per animation, 0 = disabled)
    #[arg(long, default_value = "0")]
    cycle: u32,

    /// Record animation to .asciianim file
    #[arg(long)]
    record: Option<String>,

    /// Play back a recorded .asciianim file
    #[arg(long)]
    play: Option<String>,

    /// Scale factor for particle/element counts (0.5-2.0)
    #[arg(short, long, default_value = "1.0")]
    scale: f64,

    /// Hide the status bar for pure animation mode
    #[arg(long)]
    clean: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if let Some(ref path) = cli.play {
        let player = record::Player::load(path)?;
        return player.play();
    }

    if cli.list {
        println!("Available animations:");
        for &(name, desc) in animations::ANIMATIONS {
            println!("  {:<12} {}", name, desc);
        }
        println!("\nRender modes: braille, half-block, ascii");
        println!("Color modes: mono, ansi16, ansi256, true-color");
        return Ok(());
    }

    let anim_name = cli.animation.clone().unwrap_or_else(|| "fire".to_string());
    let fps = cli.fps.clamp(1, 120);
    let frame_dur = Duration::from_secs_f64(1.0 / fps as f64);

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let result = run_loop(&cli, &anim_name, frame_dur);

    let mut stdout = io::stdout();
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

const RENDER_MODES: [RenderMode; 3] = [RenderMode::Braille, RenderMode::HalfBlock, RenderMode::Ascii];
const COLOR_MODES: [ColorMode; 4] = [ColorMode::TrueColor, ColorMode::Ansi256, ColorMode::Ansi16, ColorMode::Mono];

fn run_loop(
    cli: &Cli,
    initial_anim: &str,
    frame_dur: Duration,
) -> io::Result<()> {
    let (mut cols, mut rows) = terminal::size()?;

    let explicit_render = cli.render;
    let mut color_mode = cli.color;
    let mut hide_status = cli.clean;
    let scale = cli.scale.clamp(0.5, 2.0);

    let display_rows = if hide_status { rows as usize } else { (rows as usize).saturating_sub(1) };
    let temp_canvas = Canvas::new(cols as usize, display_rows, RenderMode::HalfBlock, color_mode);
    let mut anim: Box<dyn Animation> = animations::create(initial_anim, temp_canvas.width, temp_canvas.height, scale);
    let mut render_mode = explicit_render.unwrap_or_else(|| anim.preferred_render());
    let mut canvas = Canvas::new(cols as usize, display_rows, render_mode, color_mode);
    anim = animations::create(initial_anim, canvas.width, canvas.height, scale);

    let mut anim_index = animations::ANIMATION_NAMES.iter()
        .position(|&n| n == initial_anim)
        .unwrap_or(0);

    let start = Instant::now();
    let mut last_frame = Instant::now();
    let mut cycle_start = Instant::now();
    let mut frame_count: u64 = 0;
    let mut actual_fps: f64 = 0.0;
    let mut fps_update = Instant::now();
    let mut recorder = cli.record.as_ref().map(|_| record::Recorder::new());
    let mut needs_rebuild = false;
    // Manual frame buffer — we control when it gets written
    let mut frame_buf: Vec<u8> = Vec::with_capacity(256 * 1024);
    // Resize cooldown — skip frames after resize
    let mut resize_cooldown = Instant::now();

    loop {
        // Use event::poll as frame timer — properly yields to OS for signal handling
        let time_to_next = frame_dur.saturating_sub(last_frame.elapsed());
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
                    Event::Key(KeyEvent { code, .. }) => match code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if let (Some(rec), Some(path)) = (recorder.take(), &cli.record) {
                                let mut stdout = io::stdout();
                                execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
                                terminal::disable_raw_mode()?;
                                rec.save(path)?;
                                println!("Saved {} frames to {}", rec.frame_count(), path);
                                terminal::enable_raw_mode()?;
                                execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
                            }
                            return Ok(());
                        }
                        KeyCode::Right | KeyCode::Char('n') => {
                            anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
                            anim = animations::create(
                                animations::ANIMATION_NAMES[anim_index],
                                canvas.width, canvas.height, scale,
                            );
                            if explicit_render.is_none() {
                                render_mode = anim.preferred_render();
                                needs_rebuild = true;
                            }
                            cycle_start = Instant::now();
                        }
                        KeyCode::Left | KeyCode::Char('p') => {
                            anim_index = if anim_index == 0 {
                                animations::ANIMATION_NAMES.len() - 1
                            } else {
                                anim_index - 1
                            };
                            anim = animations::create(
                                animations::ANIMATION_NAMES[anim_index],
                                canvas.width, canvas.height, scale,
                            );
                            if explicit_render.is_none() {
                                render_mode = anim.preferred_render();
                                needs_rebuild = true;
                            }
                            cycle_start = Instant::now();
                        }
                        KeyCode::Char('r') => {
                            let idx = RENDER_MODES.iter().position(|&m| m == render_mode).unwrap_or(0);
                            render_mode = RENDER_MODES[(idx + 1) % RENDER_MODES.len()];
                            needs_rebuild = true;
                        }
                        KeyCode::Char('c') => {
                            let idx = COLOR_MODES.iter().position(|&m| m == color_mode).unwrap_or(0);
                            color_mode = COLOR_MODES[(idx + 1) % COLOR_MODES.len()];
                            needs_rebuild = true;
                        }
                        KeyCode::Char('h') => {
                            hide_status = !hide_status;
                            needs_rebuild = true;
                        }
                        _ => {}
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
                let display_rows = if hide_status { rows as usize } else { (rows as usize).saturating_sub(1) };
                canvas = Canvas::new(cols as usize, display_rows, render_mode, color_mode);
                anim = animations::create(
                    animations::ANIMATION_NAMES[anim_index],
                    canvas.width, canvas.height, scale,
                );
                // Clear screen
                let mut stdout = io::stdout().lock();
                stdout.write_all(b"\x1b[2J\x1b[H")?;
                stdout.flush()?;
            }
            needs_rebuild = false;
            last_frame = Instant::now();
            continue; // Skip this frame, render fresh next iteration
        }

        // Auto-cycle
        if cli.cycle > 0 && cycle_start.elapsed() >= Duration::from_secs(cli.cycle as u64) {
            anim_index = (anim_index + 1) % animations::ANIMATION_NAMES.len();
            anim = animations::create(
                animations::ANIMATION_NAMES[anim_index],
                canvas.width, canvas.height, scale,
            );
            if explicit_render.is_none() {
                render_mode = anim.preferred_render();
                needs_rebuild = true;
            }
            cycle_start = Instant::now();
        }

        // Timing
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f64().min(0.1); // Cap dt to avoid huge jumps
        let time = start.elapsed().as_secs_f64();
        last_frame = now;

        // Update animation
        anim.update(&mut canvas, dt, time);

        // Render to string
        let frame = canvas.render();

        // Record if active
        if let Some(ref mut rec) = recorder {
            rec.capture(&frame);
        }

        // Build frame buffer with synchronized output
        frame_buf.clear();
        // Begin synchronized update — terminal batches everything until end marker
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
            let status = format!(
                " {} | {:?} | {:?} | {:.0} fps{} | [←/→] anim  [r] render  [c] color  [h] hide  [q] quit ",
                anim.name(), render_mode, color_mode, actual_fps, rec_indicator,
            );
            let w = cols as usize;
            let truncated: String = status.chars().take(w).collect();
            let padded = format!("{:<width$}", truncated, width = w);
            frame_buf.extend_from_slice(format!("\x1b[{};1H\x1b[7m{}\x1b[0m", rows, padded).as_bytes());
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

        // Write entire frame in one syscall
        let mut stdout = io::stdout().lock();
        stdout.write_all(&frame_buf)?;
        stdout.flush()?;
    }
}
