use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::{cursor, execute, terminal};

/// A single recorded frame with its timestamp.
struct Frame {
    timestamp_ms: u64,
    content: String,
}

/// Captures rendered frames with timestamps for later playback.
pub struct Recorder {
    frames: Vec<Frame>,
    start: Instant,
}

impl Recorder {
    /// Create a new Recorder.
    pub fn new() -> Self {
        Recorder {
            frames: Vec::new(),
            start: Instant::now(),
        }
    }

    /// Record a rendered frame.
    pub fn capture(&mut self, content: &str) {
        let timestamp_ms = self.start.elapsed().as_millis() as u64;
        self.frames.push(Frame {
            timestamp_ms,
            content: content.to_string(),
        });
    }

    /// Save recorded frames to a .asciianim file.
    ///
    /// Format:
    /// ```text
    /// ASCIIANIM v1
    /// FRAMES <count>
    /// ---
    /// T <timestamp_ms>
    /// <frame content (base64 encoded)>
    /// ---
    /// ...
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        use std::io::Write as _;
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "ASCIIANIM v1")?;
        writeln!(writer, "FRAMES {}", self.frames.len())?;

        for frame in &self.frames {
            writeln!(writer, "---")?;
            writeln!(writer, "T {}", frame.timestamp_ms)?;
            // Base64 encode frame content to avoid delimiter conflicts
            let encoded = base64_encode(frame.content.as_bytes());
            writeln!(writer, "{}", encoded)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Number of frames recorded.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

/// Plays back a recorded .asciianim file.
pub struct Player {
    frames: Vec<Frame>,
}

impl Player {
    /// Load a .asciianim file for playback.
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Parse header
        let header = lines
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing header"))??;
        if !header.starts_with("ASCIIANIM v1") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid header: {}", header),
            ));
        }

        let frame_count_line = lines
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing frame count"))??;
        let _frame_count: usize = frame_count_line
            .strip_prefix("FRAMES ")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid frame count"))?;

        let mut frames = Vec::new();

        while let Some(line) = lines.next() {
            let line = line?;
            if line != "---" {
                continue;
            }

            // Read timestamp
            let t_line = lines.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "Missing timestamp")
            })??;
            let timestamp_ms: u64 = t_line
                .strip_prefix("T ")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid timestamp"))?;

            // Read base64 encoded content
            let encoded = lines.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "Missing frame content")
            })??;

            let content_bytes = base64_decode(&encoded).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Base64 decode error: {}", e),
                )
            })?;
            let content = String::from_utf8(content_bytes).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("UTF-8 error: {}", e))
            })?;

            frames.push(Frame {
                timestamp_ms,
                content,
            });
        }

        Ok(Player { frames })
    }

    /// Play back the recording to the terminal.
    pub fn play(&self) -> io::Result<()> {
        if self.frames.is_empty() {
            println!("No frames to play.");
            return Ok(());
        }

        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

        let start = Instant::now();

        for frame in &self.frames {
            // Wait until the correct time
            let target = Duration::from_millis(frame.timestamp_ms);
            let elapsed = start.elapsed();
            if target > elapsed {
                std::thread::sleep(target - elapsed);
            }

            // Check for quit
            if crossterm::event::poll(Duration::ZERO)?
                && let crossterm::event::Event::Key(key) = crossterm::event::read()?
                && matches!(
                    key.code,
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc
                )
            {
                break;
            }

            execute!(stdout, cursor::MoveTo(0, 0))?;
            stdout.write_all(frame.content.as_bytes())?;
            stdout.flush()?;
        }

        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        println!(
            "Playback complete: {} frames, {:.1}s",
            self.frames.len(),
            self.frames.last().map_or(0, |f| f.timestamp_ms) as f64 / 1000.0
        );

        Ok(())
    }
}

// Simple base64 encoder/decoder (no external dependency needed)

const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(B64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(B64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(B64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(B64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
    let data: Vec<u8> = data.bytes().filter(|&b| b != b'\n' && b != b'\r').collect();
    if !data.len().is_multiple_of(4) {
        return Err("Invalid base64 length".to_string());
    }

    let mut result = Vec::with_capacity(data.len() / 4 * 3);

    for chunk in data.chunks(4) {
        let mut vals = [0u32; 4];
        for (i, &byte) in chunk.iter().enumerate() {
            vals[i] = match byte {
                b'A'..=b'Z' => (byte - b'A') as u32,
                b'a'..=b'z' => (byte - b'a' + 26) as u32,
                b'0'..=b'9' => (byte - b'0' + 52) as u32,
                b'+' => 62,
                b'/' => 63,
                b'=' => 0,
                _ => return Err(format!("Invalid base64 character: {}", byte as char)),
            };
        }

        let triple = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip_empty() {
        let input: &[u8] = b"";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base64_roundtrip_hello() {
        let input = b"hello";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base64_roundtrip_all_bytes() {
        let input: Vec<u8> = (0u8..=255u8).collect();
        let encoded = base64_encode(&input);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }
}
