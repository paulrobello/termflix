use crate::render::Canvas;
use super::Animation;
use rand::RngExt;

struct TextLine {
    text: String,
    color: (u8, u8, u8),
    age: f64,
    kind: LineKind,
}

enum LineKind {
    Normal,
    Banner,
    Warning,
    Progress { target: f64, current: f64 },
}

/// Fake hacker terminal with scrolling commands and output
pub struct Hackerman {
    lines: Vec<TextLine>,
    next_timer: f64,
    width: usize,
    height: usize,
}

const COMMANDS: &[&str] = &[
    "$ ssh root@{} -p 22",
    "$ nmap -sS -T4 {}/24",
    "$ curl -X POST https://api.{}/v2/auth",
    "$ hydra -l admin -P rockyou.txt {}",
    "$ sqlmap -u \"http://{}/login?id=1\" --dump",
    "$ john --wordlist=passwd.txt shadow.hash",
    "$ tcpdump -i eth0 -n port 443",
    "$ aircrack-ng -w dict.txt capture.cap",
    "$ msfconsole -q -x \"use exploit/multi/handler\"",
    "$ nc -lvnp 4444",
    "$ python3 exploit.py --target {}",
    "$ cat /etc/shadow | head -20",
    "$ iptables -A INPUT -s {} -j DROP",
    "$ openssl enc -aes-256-cbc -in secrets.db -out encrypted.bin",
    "$ nikto -h https://{}:8443",
    "$ gobuster dir -u http://{} -w common.txt -t 50",
    "$ hashcat -m 1000 ntlm.hash rockyou.txt --force",
    "$ wget http://{}:8080/shell.php -O /tmp/s.php",
    "$ chmod +x payload && ./payload &",
    "$ find / -perm -4000 -type f 2>/dev/null",
];

const OUTPUTS: &[&str] = &[
    "Connection established.",
    "PORT     STATE SERVICE",
    "22/tcp   open  ssh        OpenSSH 8.9",
    "80/tcp   open  http       Apache 2.4.52",
    "443/tcp  open  https",
    "3306/tcp open  mysql      MySQL 8.0.32",
    "8080/tcp open  http-proxy",
    "Discovered open port 445/tcp on {}",
    "Host is up (0.023s latency). 997 ports filtered.",
    "[+] Valid credentials found: admin:p@ssw0rd123",
    "[+] Session 1 opened ({} -> {}:52431)",
    "[*] Sending stage (175174 bytes) to {}",
    "root:$6$rAnD0m$xYz:18291:0:99999:7:::",
    "www-data:$6$aBcDeF$123:18452:0:99999:7:::",
    "[!] Firewall rule added",
    "sqlmap identified injection point(s):",
    "  Parameter: id (GET)  Type: UNION query",
    "  Database: users  Table: credentials  [47 entries]",
    "[+] Cracked: admin:sunshine2024",
    "SHA256: 9f86d08188...4f1b2b0b822cd15d6c15b0f00a08",
    "  4,218,943 bytes transferred",
    "[!] VULNERABLE -- CVE-2024-21762",
    "[*] Exploit completed, but no session created.",
    "[*] Exploit completed, session opened!",
    "Packets captured: {} | Dropped: 0",
    "DNS: {} -> 93.184.216.34",
    "TLS 1.3 handshake complete",
    "/var/log/auth.log: 247 failed login attempts from {}",
    "uid=0(root) gid=0(root) groups=0(root)",
    "[+] Reverse shell connected from {}",
    "Exfiltrating... {} rows from credentials table",
    "drwxr-xr-x  root root  /usr/bin/sudo",
    "-rwsr-xr-x  root root  /usr/bin/passwd",
    "[*] Meterpreter session 2 opened ({} -> {})",
    "  -> Migrating to PID 1284 (svchost.exe)...",
    "  -> Migration successful!",
    "[+] Dumping SAM hashes...",
    "  Administrator:500:aad3b435...::::",
    "[*] Pivoting through {} to reach 10.10.0.0/16",
    "PING {} - 64 bytes: icmp_seq=1 ttl=64 time=0.4ms",
];

const BANNERS: &[&str] = &[
    ">>> ACCESS GRANTED <<<",
    "*** FIREWALL BYPASSED ***",
    "--- ROOT ACCESS OBTAINED ---",
    "=== ENCRYPTED CHANNEL OPEN ===",
    ">>> PAYLOAD DELIVERED <<<",
    "+++ PRIVILEGE ESCALATION SUCCESS +++",
];

const WARNINGS: &[&str] = &[
    "!!! INTRUSION DETECTED !!!",
    "!!! ALERT: IDS TRIGGERED !!!",
    "!!! CONNECTION RESET BY PEER !!!",
];

fn rand_ip(rng: &mut impl rand::RngExt) -> String {
    format!("{}.{}.{}.{}",
        rng.random_range(10u8..220), rng.random_range(0u8..255),
        rng.random_range(0u8..255), rng.random_range(1u8..254))
}

impl Hackerman {
    pub fn new(width: usize, height: usize, _scale: f64) -> Self {
        Hackerman {
            lines: Vec::new(),
            next_timer: 0.0,
            width,
            height,
        }
    }

    fn gen_line(&self, rng: &mut impl rand::RngExt) -> TextLine {
        let r = rng.random_range(0.0f64..1.0);
        let ip1 = rand_ip(rng);
        let ip2 = rand_ip(rng);

        if r < 0.18 {
            // Command
            let tmpl = COMMANDS[rng.random_range(0..COMMANDS.len())];
            let text = tmpl.replace("{}", &ip1);
            TextLine { text, color: (0, 255, 0), age: 0.0, kind: LineKind::Normal }
        } else if r < 0.22 {
            // Banner
            let text = BANNERS[rng.random_range(0..BANNERS.len())].to_string();
            TextLine { text, color: (50, 255, 50), age: 0.0, kind: LineKind::Banner }
        } else if r < 0.25 {
            // Warning
            let text = WARNINGS[rng.random_range(0..WARNINGS.len())].to_string();
            TextLine { text, color: (255, 50, 50), age: 0.0, kind: LineKind::Warning }
        } else if r < 0.30 {
            // Progress
            let labels = ["Uploading payload", "Cracking hash", "Downloading DB",
                         "Decrypting", "Scanning ports", "Brute forcing"];
            let label = labels[rng.random_range(0..labels.len())];
            TextLine {
                text: label.to_string(),
                color: (0, 200, 255), age: 0.0,
                kind: LineKind::Progress { target: rng.random_range(0.6..1.0), current: 0.0 },
            }
        } else if r < 0.33 {
            // Blank
            TextLine { text: String::new(), color: (0, 0, 0), age: 0.0, kind: LineKind::Normal }
        } else {
            // Output
            let tmpl = OUTPUTS[rng.random_range(0..OUTPUTS.len())];
            let num = rng.random_range(100u32..99999);
            let text = tmpl.replace("{}", &if tmpl.matches("{}").count() > 1 {
                ip1.clone()
            } else if tmpl.contains("rows") || tmpl.contains("captured") {
                num.to_string()
            } else {
                ip1.clone()
            });
            // Second pass for templates with multiple {}
            let text = if text.contains("{}") { text.replacen("{}", &ip2, 1) } else { text };

            let color = if text.contains("[+]") || text.contains("uid=0") {
                (100, 255, 100)
            } else if text.contains("[!]") || text.contains("VULNERABLE") {
                (255, 200, 50)
            } else if text.contains("[*]") || text.contains("->") {
                (100, 180, 255)
            } else {
                let g = rng.random_range(120u8..180);
                (g / 2, g, g / 3)
            };
            TextLine { text, color, age: 0.0, kind: LineKind::Normal }
        }
    }
}

impl Animation for Hackerman {
    fn name(&self) -> &str { "hackerman" }

    fn preferred_render(&self) -> crate::render::RenderMode {
        crate::render::RenderMode::Ascii
    }

    fn update(&mut self, canvas: &mut Canvas, dt: f64, time: f64) {
        let mut rng = rand::rng();
        self.width = canvas.width;
        self.height = canvas.height;

        // Update existing progress bars
        let mut any_progress = false;
        for line in &mut self.lines {
            if let LineKind::Progress { target, ref mut current } = line.kind {
                if *current < target {
                    *current += dt * rng.random_range(0.2..0.8);
                    if *current > target { *current = target; }
                    any_progress = true;
                }
            }
            line.age += dt;
        }

        // Add new lines
        self.next_timer -= dt;
        if self.next_timer <= 0.0 && !any_progress {
            // Vary speed — fast bursts with pauses
            self.next_timer = if rng.random_range(0.0f64..1.0) < 0.1 {
                rng.random_range(0.3..0.8)
            } else {
                rng.random_range(0.02..0.10)
            };

            let line = self.gen_line(&mut rng);
            self.lines.push(line);

            // Trim old lines
            let max = self.height + 20;
            if self.lines.len() > max {
                self.lines.drain(0..(self.lines.len() - max));
            }
        }

        canvas.clear();

        // Render from bottom — most recent lines at bottom
        let visible = self.height;
        let total = self.lines.len();
        let start = total.saturating_sub(visible);

        for (i, line) in self.lines[start..].iter().enumerate() {
            if i >= canvas.height { break; }

            let (cr, cg, cb) = line.color;

            // Fade older lines
            let age_fade = (1.0 - (line.age * 0.1).min(0.5)).max(0.5);

            match &line.kind {
                LineKind::Progress { target: _, current } => {
                    // "[Label] [████████░░░░░░░░] 73%"
                    let label = format!("[{}] [", line.text);
                    let pct_str = format!("] {:.0}%", current * 100.0);
                    let bar_w = self.width.saturating_sub(label.len() + pct_str.len() + 1);
                    let filled = (*current * bar_w as f64) as usize;

                    // Write label
                    for (cx, ch) in label.chars().enumerate() {
                        if cx < canvas.width {
                            canvas.set_char(cx, i, ch, cr, cg, cb);
                        }
                    }
                    // Write bar
                    for bx in 0..bar_w {
                        let px = label.len() + bx;
                        if px < canvas.width {
                            if bx < filled {
                                canvas.set_char(px, i, '█', 0, 230, 200);
                            } else {
                                canvas.set_char(px, i, '░', 40, 40, 40);
                            }
                        }
                    }
                    // Write percent
                    for (cx, ch) in pct_str.chars().enumerate() {
                        let px = label.len() + bar_w + cx;
                        if px < canvas.width {
                            canvas.set_char(px, i, ch, cr, cg, cb);
                        }
                    }
                }
                LineKind::Banner => {
                    // Center the banner
                    let pad = self.width.saturating_sub(line.text.len()) / 2;
                    let flash = (1.0 - (line.age * 0.4).min(0.6)).max(0.4);
                    for (cx, ch) in line.text.chars().enumerate() {
                        let px = pad + cx;
                        if px < canvas.width {
                            canvas.set_char(px, i, ch,
                                (cr as f64 * flash) as u8,
                                (cg as f64 * flash) as u8,
                                (cb as f64 * flash) as u8);
                        }
                    }
                }
                LineKind::Warning => {
                    let pad = self.width.saturating_sub(line.text.len()) / 2;
                    let blink = if ((time * 4.0).sin() > 0.0) || line.age > 1.5 { 1.0 } else { 0.3 };
                    for (cx, ch) in line.text.chars().enumerate() {
                        let px = pad + cx;
                        if px < canvas.width {
                            canvas.set_char(px, i, ch,
                                (cr as f64 * blink) as u8,
                                (cg as f64 * blink) as u8,
                                (cb as f64 * blink) as u8);
                        }
                    }
                }
                LineKind::Normal => {
                    for (cx, ch) in line.text.chars().enumerate() {
                        if cx < canvas.width {
                            canvas.set_char(cx, i, ch,
                                (cr as f64 * age_fade) as u8,
                                (cg as f64 * age_fade) as u8,
                                (cb as f64 * age_fade) as u8);
                        }
                    }
                }
            }
        }

        // Blinking cursor
        let blink = (time * 3.0).sin() > 0.0;
        if blink {
            let cy = (total.saturating_sub(start)).min(canvas.height.saturating_sub(1));
            let cursor_x = if let Some(last) = self.lines.last() {
                match &last.kind {
                    LineKind::Normal | LineKind::Banner | LineKind::Warning => last.text.len() + 1,
                    LineKind::Progress { .. } => 0,
                }
            } else { 0 };
            if cursor_x < canvas.width && cy < canvas.height {
                canvas.set_char(cursor_x, cy, '█', 0, 255, 0);
            }
        }
    }
}
