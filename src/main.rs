mod animations;
pub mod generators;
mod record;
mod render;

use animations::Animation;
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use render::{Canvas, ColorMode, RenderMode};
use std::io;
use std::time::{Duration, Instant};

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

    // Write restore sequences via raw fd write (non-blocking, bypasses Rust buffering).
    // Then exit immediately. If tmux buffer is full, these may not get through,
    // but the terminal will restore state when the PTY closes anyway.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = io::stdout().as_raw_fd();
        // Make stdout non-blocking so these writes don't hang
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        let restore = b"\x1b[?25h\x1b[?1049l";
        unsafe {
            libc::write(fd, restore.as_ptr() as *const libc::c_void, restore.len());
        }
    }
    #[cfg(not(unix))]
    {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen);
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

fn run_loop(cli: &Cli, initial_anim: &str, frame_dur: Duration) -> io::Result<()> {
    let (mut cols, mut rows) = terminal::size()?;
    let is_tmux = std::env::var("TMUX").is_ok();

    let explicit_render = cli.render;
    let mut color_mode = cli.color;
    let mut hide_status = cli.clean;
    let scale = cli.scale.clamp(0.5, 2.0);
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
        animations::create(initial_anim, temp_canvas.width, temp_canvas.height, scale);
    let mut render_mode = explicit_render.unwrap_or_else(|| anim.preferred_render());
    let mut canvas = Canvas::new(cols as usize, display_rows, render_mode, color_mode);
    anim = animations::create(initial_anim, canvas.width, canvas.height, scale);

    let mut anim_index = animations::ANIMATION_NAMES
        .iter()
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
                                canvas.width,
                                canvas.height,
                                scale,
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
                                canvas.width,
                                canvas.height,
                                scale,
                            );
                            if explicit_render.is_none() {
                                render_mode = anim.preferred_render();
                                needs_rebuild = true;
                            }
                            cycle_start = Instant::now();
                        }
                        KeyCode::Char('r') => {
                            let idx = RENDER_MODES
                                .iter()
                                .position(|&m| m == render_mode)
                                .unwrap_or(0);
                            render_mode = RENDER_MODES[(idx + 1) % RENDER_MODES.len()];
                            needs_rebuild = true;
                        }
                        KeyCode::Char('c') => {
                            let idx = COLOR_MODES
                                .iter()
                                .position(|&m| m == color_mode)
                                .unwrap_or(0);
                            color_mode = COLOR_MODES[(idx + 1) % COLOR_MODES.len()];
                            needs_rebuild = true;
                        }
                        KeyCode::Char('h') => {
                            hide_status = !hide_status;
                            needs_rebuild = true;
                        }
                        _ => {}
                    },
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
                anim = animations::create(
                    animations::ANIMATION_NAMES[anim_index],
                    canvas.width,
                    canvas.height,
                    scale,
                );
                // No clear screen — next frame overwrites everything.
                // Clearing here with a blocking flush can lock up in tmux
                // when the output buffer is full from the previous frame.
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
                canvas.width,
                canvas.height,
                scale,
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
            let status = format!(
                " {} | {:?} | {:?} | {:.0} fps{} | [←/→] anim  [r] render  [c] color  [h] hide  [q] quit ",
                anim.name(),
                render_mode,
                color_mode,
                actual_fps,
                rec_indicator,
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

        // Write frame in chunks — check for quit between chunks so 'q' is responsive
        // even when tmux's buffer is full and writes block.
        let write_start = Instant::now();
        {
            use std::os::unix::io::AsRawFd;
            let fd = io::stdout().as_raw_fd();
            let mut written = 0;
            let buf = &frame_buf;
            while written < buf.len() {
                // Check for quit keypress between chunks
                if event::poll(Duration::ZERO)?
                    && let Event::Key(KeyEvent { code, .. }) = event::read()?
                    && matches!(code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    return Ok(());
                }
                // Write a chunk — may block briefly but not for the whole frame
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
                } else if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        // Adaptive frame pacing: adjust frame duration based on actual write throughput.
        // In tmux, writes block when the buffer is full, so write time reflects
        // how fast tmux can actually process our output.
        if is_tmux {
            let write_secs = write_start.elapsed().as_secs_f64();
            write_time_ema = write_time_ema * 0.8 + write_secs * 0.2;
            // Target: frame duration = write time + small margin for animation update
            // This ensures we never write faster than tmux can process
            let target =
                Duration::from_secs_f64((write_time_ema * 1.1).max(frame_dur.as_secs_f64()));
            adaptive_frame_dur = target.min(Duration::from_millis(200)); // cap at 5fps minimum
        }
    }
}
