use super::Animation;
use crate::render::Canvas;
use rand::RngExt;

struct LogLine {
    text: String,
    color: (u8, u8, u8),
    age: f64,
}

struct ProgressBar {
    label: String,
    value: f64,
    target: f64,
    speed: f64,
    color: (u8, u8, u8),
}

struct NetworkNode {
    x: f64,
    y: f64,
    label: String,
    active: bool,
    pulse: f64,
}

/// Fake movie-style hacker HUD with multiple static panels
pub struct Hackerman {
    width: usize,
    height: usize,
    // Scrolling log (bottom-left panel)
    log_lines: Vec<LogLine>,
    log_timer: f64,
    // Progress bars (bottom-right panel)
    bars: Vec<ProgressBar>,
    bar_reset_timer: f64,
    // Network map nodes (top-right panel)
    nodes: Vec<NetworkNode>,
    node_timer: f64,
    active_connection: Option<(usize, usize)>,
    conn_timer: f64,
    // Top-left stats
    stats_flicker: f64,
    packets_count: u64,
    bytes_count: u64,
    threats_count: u32,
    uptime_secs: f64,
    rng: rand::rngs::ThreadRng,
}

const LOG_MSGS: &[&str] = &[
    "[+] Session opened ({})",
    "[*] Scanning port {}...",
    "PORT {}/tcp open",
    "[+] Credentials: admin:{}",
    "[!] Firewall rule bypassed",
    "$ cat /etc/shadow",
    "root:$6$rX9:18291:0:99999:::",
    "[*] Sending payload ({}b)",
    "[+] Shell spawned on {}",
    ">>> Pivoting to {}",
    "[!] IDS alert suppressed",
    "$ chmod +x exploit",
    "[*] Extracting database...",
    "[+] {} rows dumped",
    "DNS: {} → 93.184.216.34",
    "[+] Tunnel established",
    "$ ssh -D 9050 root@{}",
    "[*] ARP spoofing gateway",
    "[+] MITM active on {}",
    "TLS intercepted (RSA-2048)",
    "[!] CVE-2024-21762 found",
    "[+] Exploit successful!",
    "uid=0(root) gid=0(root)",
    "$ wget http://{}/shell.php",
    "[*] Migrating to PID 1284",
];

const BAR_LABELS: &[&str] = &[
    "DECRYPT", "CRACK", "UPLOAD", "EXFIL", "SCAN", "INJECT", "COMPILE", "BREACH",
];

fn rand_ip(rng: &mut impl rand::RngExt) -> String {
    format!(
        "{}.{}.{}.{}",
        rng.random_range(10u8..220),
        rng.random_range(0u8..255),
        rng.random_range(0u8..255),
        rng.random_range(1u8..254)
    )
}

fn rand_word(rng: &mut impl rand::RngExt) -> String {
    let words = [
        "alpha", "omega", "delta", "ghost", "nexus", "cipher", "shadow", "void", "zero", "root",
    ];
    words[rng.random_range(0..words.len())].to_string()
}

impl Hackerman {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        let mut rng = rand::rng();
        let node_names = [
            "GATEWAY", "FIREWALL", "DB-01", "APP-SRV", "DNS", "PROXY", "TARGET", "C2",
        ];
        let nodes: Vec<NetworkNode> = node_names
            .iter()
            .enumerate()
            .map(|(i, name)| NetworkNode {
                x: rng.random_range(0.1..0.9),
                y: rng.random_range(0.1..0.9),
                label: name.to_string(),
                active: i == 0,
                pulse: rng.random_range(0.0..std::f64::consts::TAU),
            })
            .collect();

        let bars = (0..4)
            .map(|_| {
                let label = BAR_LABELS[rng.random_range(0..BAR_LABELS.len())];
                ProgressBar {
                    label: label.to_string(),
                    value: 0.0,
                    target: rng.random_range(0.7..1.0),
                    speed: rng.random_range(0.05..0.25),
                    color: match rng.random_range(0u8..3) {
                        0 => (0, 220, 180),
                        1 => (0, 180, 255),
                        _ => (0, 255, 100),
                    },
                }
            })
            .collect();

        Hackerman {
            width,
            height,
            log_lines: Vec::new(),
            log_timer: 0.0,
            bars,
            bar_reset_timer: 0.0,
            nodes,
            node_timer: 0.0,
            active_connection: Some((0, 1)),
            conn_timer: 0.0,
            stats_flicker: 0.0,
            packets_count: 48291,
            bytes_count: 1_284_019,
            threats_count: 3,
            uptime_secs: 3847.0,
            rng: rand::rng(),
        }
    }
}

impl Animation for Hackerman {
    fn name(&self) -> &str {
        "hackerman"
    }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Ascii
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        self.uptime_secs += dt;

        if self.width < 40 || self.height < 15 {
            return;
        }

        canvas.clear();

        // Layout: split into quadrants
        let mid_x = self.width / 2;
        let mid_y = self.height / 2;

        // ── Draw borders ──
        let border_color: (u8, u8, u8) = (0, 100, 60);
        // Horizontal divider
        for x in 0..self.width {
            canvas.set_char(
                x,
                mid_y,
                '─',
                border_color.0,
                border_color.1,
                border_color.2,
            );
            canvas.set_char(x, 0, '─', border_color.0, border_color.1, border_color.2);
            if self.height > 1 {
                canvas.set_char(
                    x,
                    self.height - 1,
                    '─',
                    border_color.0,
                    border_color.1,
                    border_color.2,
                );
            }
        }
        // Vertical divider
        for y in 0..self.height {
            canvas.set_char(
                mid_x,
                y,
                '│',
                border_color.0,
                border_color.1,
                border_color.2,
            );
            canvas.set_char(0, y, '│', border_color.0, border_color.1, border_color.2);
            if self.width > 1 {
                canvas.set_char(
                    self.width - 1,
                    y,
                    '│',
                    border_color.0,
                    border_color.1,
                    border_color.2,
                );
            }
        }
        // Corners and intersections
        canvas.set_char(0, 0, '┌', border_color.0, border_color.1, border_color.2);
        canvas.set_char(
            self.width - 1,
            0,
            '┐',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            0,
            self.height - 1,
            '└',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            self.width - 1,
            self.height - 1,
            '┘',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            mid_x,
            0,
            '┬',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            mid_x,
            self.height - 1,
            '┴',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            0,
            mid_y,
            '├',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            self.width - 1,
            mid_y,
            '┤',
            border_color.0,
            border_color.1,
            border_color.2,
        );
        canvas.set_char(
            mid_x,
            mid_y,
            '┼',
            border_color.0,
            border_color.1,
            border_color.2,
        );

        // ── Panel titles ──
        draw_text(canvas, 2, 0, "[ SYSTEM STATUS ]", (0, 200, 100));
        draw_text(canvas, mid_x + 2, 0, "[ NETWORK MAP ]", (0, 200, 100));
        draw_text(canvas, 2, mid_y, "[ ACTIVITY LOG ]", (0, 200, 100));
        draw_text(canvas, mid_x + 2, mid_y, "[ OPERATIONS ]", (0, 200, 100));

        // ══════════════════════════════════════
        // TOP-LEFT: System Status
        // ══════════════════════════════════════
        self.stats_flicker += dt;
        self.packets_count += self.rng.random_range(10..200) as u64;
        self.bytes_count += self.rng.random_range(500..50000) as u64;
        if self.rng.random_range(0.0..1.0) < 0.005 {
            self.threats_count += 1;
        }

        let uptime_h = (self.uptime_secs / 3600.0) as u32;
        let uptime_m = ((self.uptime_secs % 3600.0) / 60.0) as u32;
        let uptime_s = (self.uptime_secs % 60.0) as u32;

        let stats_x = 2;
        let stats_y = 2;
        let dim_green: (u8, u8, u8) = (0, 160, 80);
        let bright_green: (u8, u8, u8) = (0, 255, 120);

        draw_text(canvas, stats_x, stats_y, "UPTIME:", dim_green);
        draw_text(
            canvas,
            stats_x + 10,
            stats_y,
            &format!("{:02}:{:02}:{:02}", uptime_h, uptime_m, uptime_s),
            bright_green,
        );

        draw_text(canvas, stats_x, stats_y + 2, "PACKETS:", dim_green);
        draw_text(
            canvas,
            stats_x + 10,
            stats_y + 2,
            &format!("{}", self.packets_count),
            bright_green,
        );

        draw_text(canvas, stats_x, stats_y + 4, "BYTES TX:", dim_green);
        draw_text(
            canvas,
            stats_x + 10,
            stats_y + 4,
            &format!("{}", self.bytes_count),
            bright_green,
        );

        draw_text(canvas, stats_x, stats_y + 6, "THREATS:", dim_green);
        let threat_color = if self.threats_count > 5 {
            (255, 50, 50)
        } else {
            (255, 200, 50)
        };
        draw_text(
            canvas,
            stats_x + 10,
            stats_y + 6,
            &format!("{}", self.threats_count),
            threat_color,
        );

        draw_text(canvas, stats_x, stats_y + 8, "STATUS:", dim_green);
        let blink = (time * 2.0).sin() > 0.0;
        if blink {
            draw_text(canvas, stats_x + 10, stats_y + 8, "● ACTIVE", (0, 255, 0));
        } else {
            draw_text(canvas, stats_x + 10, stats_y + 8, "● ACTIVE", (0, 120, 0));
        }

        // CPU/MEM bars
        if stats_y + 11 < mid_y {
            let cpu = 0.3 + (time * 0.7).sin().abs() * 0.5 + self.rng.random_range(0.0..0.1);
            let mem = 0.6 + (time * 0.1).sin() * 0.1;
            draw_text(canvas, stats_x, stats_y + 10, "CPU:", dim_green);
            draw_mini_bar(
                canvas,
                stats_x + 6,
                stats_y + 10,
                20,
                cpu.min(1.0),
                (0, 200, 100),
            );
            draw_text(canvas, stats_x, stats_y + 12, "MEM:", dim_green);
            draw_mini_bar(
                canvas,
                stats_x + 6,
                stats_y + 12,
                20,
                mem.min(1.0),
                (0, 180, 220),
            );
        }

        // ══════════════════════════════════════
        // TOP-RIGHT: Network Map
        // ══════════════════════════════════════
        let map_x = mid_x + 2;
        let map_y = 2;
        let map_w = self.width.saturating_sub(mid_x + 4);
        let map_h = mid_y.saturating_sub(3);

        // Update nodes
        self.node_timer += dt;
        for node in &mut self.nodes {
            node.pulse += dt * 3.0;
        }

        // Cycle active connections
        self.conn_timer += dt;
        if self.conn_timer > 2.0 {
            self.conn_timer = 0.0;
            if self.rng.random_range(0.0..1.0) < 0.4 {
                let a = self.rng.random_range(0..self.nodes.len());
                let mut b = self.rng.random_range(0..self.nodes.len());
                while b == a {
                    b = self.rng.random_range(0..self.nodes.len());
                }
                self.active_connection = Some((a, b));
                self.nodes[b].active = true;
            }
            // Randomly deactivate a node
            if self.rng.random_range(0.0..1.0) < 0.2 && self.nodes.len() > 2 {
                let idx = self.rng.random_range(1..self.nodes.len());
                self.nodes[idx].active = self.rng.random_range(0.0..1.0) < 0.5;
            }
        }

        // Draw connections (lines between nodes)
        if let Some((a, b)) = self.active_connection
            && a < self.nodes.len()
            && b < self.nodes.len()
        {
            let ax = map_x + (self.nodes[a].x * map_w as f64) as usize;
            let ay = map_y + (self.nodes[a].y * map_h as f64) as usize;
            let bx = map_x + (self.nodes[b].x * map_w as f64) as usize;
            let by = map_y + (self.nodes[b].y * map_h as f64) as usize;
            // Simple line drawing
            let steps = ((bx as f64 - ax as f64)
                .abs()
                .max((by as f64 - ay as f64).abs())) as usize;
            if steps > 0 {
                let pulse_pos = ((time * 4.0) % 1.0 * steps as f64) as usize;
                for s in 0..steps {
                    let px = ax as f64 + (bx as f64 - ax as f64) * s as f64 / steps as f64;
                    let py = ay as f64 + (by as f64 - ay as f64) * s as f64 / steps as f64;
                    let px = px as usize;
                    let py = py as usize;
                    if px < self.width && py < self.height {
                        let near_pulse = (s as i32 - pulse_pos as i32).unsigned_abs() < 3;
                        if near_pulse {
                            canvas.set_char(px, py, '●', 0, 255, 100);
                        } else {
                            canvas.set_char(px, py, '·', 0, 80, 50);
                        }
                    }
                }
            }
        }

        // Draw nodes
        for node in &self.nodes {
            let nx = map_x + (node.x * map_w as f64) as usize;
            let ny = map_y + (node.y * map_h as f64) as usize;
            if nx < self.width.saturating_sub(node.label.len() + 3) && ny < mid_y.saturating_sub(1)
            {
                let pulse_bright = if node.active {
                    0.6 + (node.pulse.sin() * 0.4).abs()
                } else {
                    0.3
                };
                let (r, g, b) = if node.active {
                    (
                        (50.0 * pulse_bright) as u8,
                        (255.0 * pulse_bright) as u8,
                        (100.0 * pulse_bright) as u8,
                    )
                } else {
                    (80, 80, 80)
                };
                let icon = if node.active { '◉' } else { '○' };
                canvas.set_char(nx, ny, icon, r, g, b);
                draw_text(canvas, nx + 2, ny, &node.label, (r, g, b));
            }
        }

        // ══════════════════════════════════════
        // BOTTOM-LEFT: Activity Log (scrolling)
        // ══════════════════════════════════════
        let log_x = 2;
        let log_y_start = mid_y + 2;
        let log_h = self.height.saturating_sub(mid_y + 3);
        let log_w = mid_x.saturating_sub(3);

        // Add new log lines
        self.log_timer -= dt;
        if self.log_timer <= 0.0 {
            self.log_timer = self.rng.random_range(0.1..0.5);
            let tmpl = LOG_MSGS[self.rng.random_range(0..LOG_MSGS.len())];
            let ip = rand_ip(&mut self.rng);
            let num = self.rng.random_range(1000u32..65535);
            let word = rand_word(&mut self.rng);
            let text = tmpl.replace(
                "{}",
                &if tmpl.contains("port") || tmpl.contains("bytes") || tmpl.contains("rows") {
                    num.to_string()
                } else if tmpl.contains("admin:{}") {
                    word
                } else {
                    ip
                },
            );

            let color = if text.contains("[+]") {
                (100, 255, 100)
            } else if text.contains("[!]") {
                (255, 200, 50)
            } else if text.contains("[*]") {
                (100, 180, 255)
            } else if text.starts_with("$") {
                (0, 255, 0)
            } else {
                (0, 180, 80)
            };

            self.log_lines.push(LogLine {
                text,
                color,
                age: 0.0,
            });
            if self.log_lines.len() > 200 {
                self.log_lines.drain(0..100);
            }
        }

        for line in &mut self.log_lines {
            line.age += dt;
        }

        // Render log (most recent at bottom)
        let visible = log_h.min(self.log_lines.len());
        let start = self.log_lines.len().saturating_sub(visible);
        for (i, line) in self.log_lines[start..].iter().enumerate() {
            let sy = log_y_start + i;
            if sy >= self.height.saturating_sub(1) {
                break;
            }
            let fade = (1.0 - (line.age * 0.08).min(0.4)).max(0.6);
            let (cr, cg, cb) = line.color;
            for (cx, ch) in line.text.chars().enumerate() {
                if cx >= log_w {
                    break;
                }
                canvas.set_char(
                    log_x + cx,
                    sy,
                    ch,
                    (cr as f64 * fade) as u8,
                    (cg as f64 * fade) as u8,
                    (cb as f64 * fade) as u8,
                );
            }
        }

        // ══════════════════════════════════════
        // BOTTOM-RIGHT: Operations (progress bars + status)
        // ══════════════════════════════════════
        let ops_x = mid_x + 2;
        let ops_y = mid_y + 2;
        let ops_w = self.width.saturating_sub(mid_x + 4);

        // Update progress bars
        self.bar_reset_timer += dt;
        for bar in &mut self.bars {
            if bar.value < bar.target {
                bar.value += bar.speed * dt;
                if bar.value > bar.target {
                    bar.value = bar.target;
                }
            }
        }

        // Reset completed bars periodically
        if self.bar_reset_timer > 5.0 {
            self.bar_reset_timer = 0.0;
            for bar in &mut self.bars {
                if bar.value >= bar.target {
                    bar.label = BAR_LABELS[self.rng.random_range(0..BAR_LABELS.len())].to_string();
                    bar.value = 0.0;
                    bar.target = self.rng.random_range(0.6..1.0);
                    bar.speed = self.rng.random_range(0.05..0.3);
                    bar.color = match self.rng.random_range(0u8..3) {
                        0 => (0, 220, 180),
                        1 => (0, 180, 255),
                        _ => (0, 255, 100),
                    };
                }
            }
        }

        // Draw progress bars
        for (i, bar) in self.bars.iter().enumerate() {
            let by = ops_y + i * 3;
            if by + 1 >= self.height.saturating_sub(1) {
                break;
            }

            // Label + percentage
            let pct = format!(" {:.0}%", bar.value * 100.0);
            let status = if bar.value >= bar.target { " ✓" } else { "" };
            draw_text(canvas, ops_x, by, &bar.label, bar.color);
            let status_color = if bar.value >= bar.target {
                (0, 255, 0)
            } else {
                bar.color
            };
            draw_text(
                canvas,
                ops_x + bar.label.len(),
                by,
                &format!("{}{}", pct, status),
                status_color,
            );

            // Bar
            let bar_w = ops_w.saturating_sub(2);
            let filled = (bar.value * bar_w as f64) as usize;
            for bx in 0..bar_w {
                let px = ops_x + bx;
                if px < self.width.saturating_sub(1) {
                    if bx < filled {
                        canvas.set_char(px, by + 1, '█', bar.color.0, bar.color.1, bar.color.2);
                    } else {
                        canvas.set_char(px, by + 1, '░', 30, 30, 30);
                    }
                }
            }
        }

        // Blinking cursor in log panel
        let blink = (time * 3.0).sin() > 0.0;
        if blink {
            let cy = log_y_start + visible.min(log_h.saturating_sub(1));
            if cy < self.height.saturating_sub(1) {
                canvas.set_char(log_x, cy, '█', 0, 255, 0);
            }
        }
    }
}

fn draw_text(canvas: &mut Canvas, x: usize, y: usize, text: &str, color: (u8, u8, u8)) {
    for (i, ch) in text.chars().enumerate() {
        let px = x + i;
        if px < canvas.width && y < canvas.height {
            canvas.set_char(px, y, ch, color.0, color.1, color.2);
        }
    }
}

fn draw_mini_bar(
    canvas: &mut Canvas,
    x: usize,
    y: usize,
    width: usize,
    value: f64,
    color: (u8, u8, u8),
) {
    let filled = (value * width as f64) as usize;
    for i in 0..width {
        let px = x + i;
        if px < canvas.width && y < canvas.height {
            if i < filled {
                canvas.set_char(px, y, '▮', color.0, color.1, color.2);
            } else {
                canvas.set_char(px, y, '▯', 40, 40, 40);
            }
        }
    }
    // Percentage
    let pct = format!(" {:.0}%", value * 100.0);
    draw_text(canvas, x + width, y, &pct, color);
}
