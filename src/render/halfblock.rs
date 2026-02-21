use super::canvas::{Canvas, ColorMode, color_to_fg};
use crossterm::style::Color;

/// Half-block rendering: each terminal cell shows 2 vertical pixels
/// using ▀ (upper half) with fg=top color, bg=bottom color

fn color_to_bg(color: Color) -> String {
    match color {
        Color::Rgb { r, g, b } => format!("48;2;{};{};{}", r, g, b),
        Color::AnsiValue(v) => format!("48;5;{}", v),
        Color::Black => "40".into(),
        Color::DarkRed => "41".into(),
        Color::DarkGreen => "42".into(),
        Color::DarkYellow => "43".into(),
        Color::DarkBlue => "44".into(),
        Color::DarkMagenta => "45".into(),
        Color::DarkCyan => "46".into(),
        Color::Grey => "47".into(),
        Color::DarkGrey => "100".into(),
        Color::Red => "101".into(),
        Color::Green => "102".into(),
        Color::Yellow => "103".into(),
        Color::Blue => "104".into(),
        Color::Magenta => "105".into(),
        Color::Cyan => "106".into(),
        Color::White => "107".into(),
        _ => "40".into(),
    }
}

pub fn render(canvas: &Canvas) -> String {
    let term_cols = canvas.width;
    let term_rows = canvas.height / 2;
    let mut out = String::with_capacity(term_cols * term_rows * 15);

    let mut last_fg = String::new();
    let mut last_bg = String::new();

    for row in 0..term_rows {
        for col in 0..term_cols {
            let top_y = row * 2;
            let bot_y = row * 2 + 1;
            let top_idx = top_y * canvas.width + col;
            let bot_idx = bot_y * canvas.width + col;

            let top_v = canvas.pixels[top_idx];
            let bot_v = canvas.pixels[bot_idx];

            if canvas.color_mode == ColorMode::Mono {
                let top_on = top_v > 0.3;
                let bot_on = bot_v > 0.3;
                match (top_on, bot_on) {
                    (true, true) => out.push('█'),
                    (true, false) => out.push('▀'),
                    (false, true) => out.push('▄'),
                    (false, false) => out.push(' '),
                }
            } else {
                let (tr, tg, tb) = canvas.colors[top_idx];
                let (br, bg, bb) = canvas.colors[bot_idx];

                let scale = |c: u8, v: f64| -> u8 { (c as f64 * v.clamp(0.0, 1.0)) as u8 };
                let top_color = canvas.map_color(scale(tr, top_v), scale(tg, top_v), scale(tb, top_v));
                let bot_color = canvas.map_color(scale(br, bot_v), scale(bg, bot_v), scale(bb, bot_v));

                let fg = color_to_fg(top_color);
                let bg_s = color_to_bg(bot_color);

                // Only emit escape sequences when colors change
                let fg_changed = fg != last_fg;
                let bg_changed = bg_s != last_bg;

                if fg_changed && bg_changed {
                    out.push_str("\x1b[");
                    out.push_str(&fg);
                    out.push(';');
                    out.push_str(&bg_s);
                    out.push('m');
                } else if fg_changed {
                    out.push_str("\x1b[");
                    out.push_str(&fg);
                    out.push('m');
                } else if bg_changed {
                    out.push_str("\x1b[");
                    out.push_str(&bg_s);
                    out.push('m');
                }

                if fg_changed { last_fg = fg; }
                if bg_changed { last_bg = bg_s; }

                out.push('▀');
            }
        }
        // Reset at end of row and move to next
        out.push_str("\x1b[0m\x1b[");
        let next_row = row + 2;
        out.push_str(&next_row.to_string());
        out.push_str(";1H");
        last_fg.clear();
        last_bg.clear();
    }
    out
}
