use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::{cursor, execute, terminal};

use crate::color::color_to_rgb;
use crate::render::cell::{Cell, CellGrid};
use crossterm::style::Color;

/// Upper bounds for a v2 recording's declared size and frame count
/// (SEC-001/SEC-008 class: a crafted header must not be able to request a
/// huge allocation before any frame data is read).
pub const MAX_COLS: usize = 1000;
pub const MAX_ROWS: usize = 500;
pub const MAX_FRAMES: usize = 100_000;

/// A single v1 recorded frame: timestamped ANSI text.
pub struct Frame {
    pub timestamp_ms: u64,
    pub content: String,
}

/// A single v2 recorded frame: timestamped cell grid.
pub struct GridFrame {
    pub timestamp_ms: u64,
    pub grid: CellGrid,
}

/// The frames of a loaded recording, by format version.
pub enum Frames {
    V1(Vec<Frame>),
    V2 {
        cols: usize,
        rows: usize,
        frames: Vec<GridFrame>,
    },
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

// ---------------------------------------------------------------------------
// v2 cell-grid binary encoding
//
// A frame is a sequence of runs. Each run is:
//   count  u16 LE      — number of identical consecutive cells (>= 1)
//   flags  u8          — bit 0: fg present, bit 1: bg present
//   len    u8          — UTF-8 byte length of the cell's char (1..=4)
//   char   [len] u8    — the char's UTF-8 bytes (exactly one char)
//   fg     [3] u8      — present when flags bit 0 set
//   bg     [3] u8      — present when flags bit 1 set
//
// Colors are stored as RGB triples regardless of the color mode that
// produced them; decode always yields `Color::Rgb`.
// ---------------------------------------------------------------------------

fn encode_grid(grid: &CellGrid) -> Vec<u8> {
    let mut out = Vec::with_capacity(grid.cells.len() * 2);
    let mut i = 0usize;
    while i < grid.cells.len() {
        let cell = grid.cells[i];
        let mut run = 1usize;
        while run < u16::MAX as usize && i + run < grid.cells.len() && grid.cells[i + run] == cell {
            run += 1;
        }
        let mut char_buf = [0u8; 4];
        let ch = cell.ch.encode_utf8(&mut char_buf);
        let flags = cell.fg.is_some() as u8 | ((cell.bg.is_some() as u8) << 1);
        out.extend_from_slice(&(run as u16).to_le_bytes());
        out.push(flags);
        out.push(ch.len() as u8);
        out.extend_from_slice(ch.as_bytes());
        if let Some(c) = cell.fg {
            let (r, g, b) = color_to_rgb(c);
            out.push(r);
            out.push(g);
            out.push(b);
        }
        if let Some(c) = cell.bg {
            let (r, g, b) = color_to_rgb(c);
            out.push(r);
            out.push(g);
            out.push(b);
        }
        i += run;
    }
    out
}

fn decode_grid(data: &[u8], cols: usize, rows: usize) -> io::Result<CellGrid> {
    let total = cols
        .checked_mul(rows)
        .ok_or_else(|| invalid("grid size overflow"))?;
    let mut cells: Vec<Cell> = Vec::with_capacity(total);
    let mut i = 0usize;
    while i < data.len() {
        if i + 4 > data.len() {
            return Err(invalid("truncated run header"));
        }
        let count = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        let flags = data[i + 2];
        let ch_len = data[i + 3] as usize;
        if count == 0 {
            return Err(invalid("zero run length"));
        }
        if ch_len == 0 || ch_len > 4 {
            return Err(invalid("invalid char length"));
        }
        i += 4;
        if i + ch_len > data.len() {
            return Err(invalid("truncated char"));
        }
        let ch_str =
            std::str::from_utf8(&data[i..i + ch_len]).map_err(|_| invalid("invalid char UTF-8"))?;
        let mut chars = ch_str.chars();
        let ch = chars.next().ok_or_else(|| invalid("empty char"))?;
        if chars.next().is_some() {
            return Err(invalid("char length does not match one char"));
        }
        i += ch_len;

        let mut fg = None;
        if flags & 0b01 != 0 {
            if i + 3 > data.len() {
                return Err(invalid("truncated fg"));
            }
            fg = Some(Color::Rgb {
                r: data[i],
                g: data[i + 1],
                b: data[i + 2],
            });
            i += 3;
        }
        let mut bg = None;
        if flags & 0b10 != 0 {
            if i + 3 > data.len() {
                return Err(invalid("truncated bg"));
            }
            bg = Some(Color::Rgb {
                r: data[i],
                g: data[i + 1],
                b: data[i + 2],
            });
            i += 3;
        }

        if cells.len() + count > total {
            return Err(invalid("run exceeds grid size"));
        }
        let cell = Cell { ch, fg, bg };
        for _ in 0..count {
            cells.push(cell);
        }
    }
    if cells.len() != total {
        return Err(invalid("cell count mismatch"));
    }
    Ok(CellGrid { cols, rows, cells })
}

/// Crop/pad a grid to `cols` x `rows` (recording through a resize: the v2
/// header carries one size, so frames are fitted to the first frame's dims).
fn fit_grid(grid: &CellGrid, cols: usize, rows: usize) -> CellGrid {
    let mut out = CellGrid::new(cols, rows);
    for r in 0..rows.min(grid.rows) {
        for c in 0..cols.min(grid.cols) {
            out.cells[r * cols + c] = grid.cells[r * grid.cols + c];
        }
    }
    out
}

/// Captures rendered cell grids with timestamps for later playback.
pub struct Recorder {
    frames: Vec<(u64, Vec<u8>)>,
    cols: usize,
    rows: usize,
    start: Instant,
}

impl Recorder {
    /// Create a new Recorder.
    pub fn new() -> Self {
        Recorder {
            frames: Vec::new(),
            cols: 0,
            rows: 0,
            start: Instant::now(),
        }
    }

    /// Record a rendered frame's cell grid. The first captured grid fixes the
    /// recording's size; later grids (e.g. after a terminal resize) are
    /// cropped or padded to match. Capture stops silently at `MAX_FRAMES` so
    /// a very long session still produces a loadable file.
    pub fn capture_grid(&mut self, grid: &CellGrid) {
        if self.frames.len() >= MAX_FRAMES {
            return;
        }
        if self.cols == 0 {
            self.cols = grid.cols.min(MAX_COLS);
            self.rows = grid.rows.min(MAX_ROWS);
        }
        let timestamp_ms = self.start.elapsed().as_millis() as u64;
        if grid.cols == self.cols && grid.rows == self.rows {
            self.frames.push((timestamp_ms, encode_grid(grid)));
        } else {
            let fitted = fit_grid(grid, self.cols, self.rows);
            self.frames.push((timestamp_ms, encode_grid(&fitted)));
        }
    }

    /// Save recorded frames to a `.asciianim` file (v2 format).
    ///
    /// Format:
    /// ```text
    /// ASCIIANIM v2
    /// SIZE <cols> <rows>
    /// FRAMES <count>
    /// ---
    /// T <timestamp_ms>
    /// <frame grid (binary cell encoding, base64 wrapped)>
    /// ---
    /// ...
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "ASCIIANIM v2")?;
        writeln!(writer, "SIZE {} {}", self.cols, self.rows)?;
        writeln!(writer, "FRAMES {}", self.frames.len())?;

        for &(timestamp_ms, ref data) in &self.frames {
            writeln!(writer, "---")?;
            writeln!(writer, "T {}", timestamp_ms)?;
            writeln!(writer, "{}", base64_encode(data))?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Number of frames recorded.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

/// Plays back a recorded .asciianim file (v1 or v2).
pub struct Player {
    frames: Frames,
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
        if header.starts_with("ASCIIANIM v2") {
            let frames = Self::load_v2(lines)?;
            return Ok(Player { frames });
        }
        if !header.starts_with("ASCIIANIM v1") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid header: {}", header),
            ));
        }
        let frames = Self::load_v1(lines)?;
        Ok(Player {
            frames: Frames::V1(frames),
        })
    }

    fn load_v1(mut lines: std::io::Lines<BufReader<File>>) -> io::Result<Vec<Frame>> {
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

        Ok(frames)
    }

    fn load_v2(mut lines: std::io::Lines<BufReader<File>>) -> io::Result<Frames> {
        let size_line = lines
            .next()
            .ok_or_else(|| invalid("v2 recording missing SIZE"))??;
        let size_rest = size_line
            .strip_prefix("SIZE ")
            .ok_or_else(|| invalid("v2 recording missing SIZE"))?;
        let mut size_parts = size_rest.split_whitespace();
        let cols: usize = size_parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| invalid("invalid SIZE"))?;
        let rows: usize = size_parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| invalid("invalid SIZE"))?;
        if size_parts.next().is_some() {
            return Err(invalid("invalid SIZE"));
        }
        if cols == 0 || rows == 0 || cols > MAX_COLS || rows > MAX_ROWS {
            return Err(invalid(&format!(
                "SIZE {cols}x{rows} out of range (max {MAX_COLS}x{MAX_ROWS})"
            )));
        }

        let frame_count_line = lines
            .next()
            .ok_or_else(|| invalid("v2 recording missing FRAMES"))??;
        let _frame_count: usize = frame_count_line
            .strip_prefix("FRAMES ")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| invalid("invalid frame count"))?;
        if _frame_count > MAX_FRAMES {
            return Err(invalid("frame count exceeds limit"));
        }

        let mut frames = Vec::new();

        while let Some(line) = lines.next() {
            let line = line?;
            if line != "---" {
                continue;
            }

            let t_line = lines.next().ok_or_else(|| invalid("missing timestamp"))??;
            let timestamp_ms: u64 = t_line
                .strip_prefix("T ")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| invalid("invalid timestamp"))?;

            let encoded = lines
                .next()
                .ok_or_else(|| invalid("missing frame content"))??;
            let data = base64_decode(&encoded)
                .map_err(|e| invalid(&format!("Base64 decode error: {}", e)))?;
            let grid = decode_grid(&data, cols, rows)?;
            frames.push(GridFrame { timestamp_ms, grid });
            if frames.len() > MAX_FRAMES {
                return Err(invalid("frame count exceeds limit"));
            }
        }

        Ok(Frames::V2 { cols, rows, frames })
    }

    /// Access the recorded frames.
    pub fn frames(&self) -> &Frames {
        &self.frames
    }

    /// True when the recording holds no frames.
    pub fn is_empty(&self) -> bool {
        match &self.frames {
            Frames::V1(fs) => fs.is_empty(),
            Frames::V2 { frames, .. } => frames.is_empty(),
        }
    }

    /// Play back the recording to the terminal. v1 frames are written
    /// verbatim; v2 grids are re-encoded through the standard full-frame
    /// encoder, so playback emits only escape sequences termflix itself
    /// produces.
    pub fn play(&self) -> io::Result<()> {
        if self.is_empty() {
            println!("No frames to play.");
            return Ok(());
        }

        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

        let start = Instant::now();

        // Lazy per-format content stream: v1 hands over the stored text,
        // v2 encodes each grid on demand.
        let seq: Box<dyn Iterator<Item = (u64, String)>> = match &self.frames {
            Frames::V1(fs) => Box::new(fs.iter().map(|f| (f.timestamp_ms, f.content.clone()))),
            Frames::V2 { frames, .. } => Box::new(frames.iter().map(|f| {
                (
                    f.timestamp_ms,
                    crate::render::encoder::encode_full(&f.grid, false),
                )
            })),
        };

        for (timestamp_ms, content) in seq {
            // Wait until the correct time
            let target = Duration::from_millis(timestamp_ms);
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
            stdout.write_all(content.as_bytes())?;
            stdout.flush()?;
        }

        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        let frame_count = match &self.frames {
            Frames::V1(fs) => fs.len(),
            Frames::V2 { frames, .. } => frames.len(),
        };
        let last_ts = match &self.frames {
            Frames::V1(fs) => fs.last().map_or(0, |f| f.timestamp_ms),
            Frames::V2 { frames, .. } => frames.last().map_or(0, |f| f.timestamp_ms),
        };
        println!(
            "Playback complete: {} frames, {:.1}s",
            frame_count,
            last_ts as f64 / 1000.0
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

    fn cell(ch: char, fg: Option<Color>, bg: Option<Color>) -> Cell {
        Cell { ch, fg, bg }
    }

    fn rgb(r: u8, g: u8, b: u8) -> Option<Color> {
        Some(Color::Rgb { r, g, b })
    }

    fn roundtrip(grid: &CellGrid) -> CellGrid {
        decode_grid(&encode_grid(grid), grid.cols, grid.rows).expect("roundtrip decode")
    }

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("termflix-test-{}-{}", std::process::id(), name));
        p
    }

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

    // ---- encode_grid / decode_grid round-trips ----

    #[test]
    fn grid_roundtrip_empty() {
        let grid = CellGrid {
            cols: 0,
            rows: 0,
            cells: Vec::new(),
        };
        assert_eq!(encode_grid(&grid), Vec::<u8>::new());
        let out = roundtrip(&grid);
        assert_eq!(out.cols, 0);
        assert!(out.cells.is_empty());
    }

    #[test]
    fn grid_roundtrip_single_cell() {
        let grid = CellGrid {
            cols: 1,
            rows: 1,
            cells: vec![cell('⣿', rgb(255, 0, 128), rgb(1, 2, 3))],
        };
        let out = roundtrip(&grid);
        assert_eq!(out.cells[0].ch, '⣿');
        assert_eq!(out.cells[0].fg, rgb(255, 0, 128));
        assert_eq!(out.cells[0].bg, rgb(1, 2, 3));
    }

    #[test]
    fn grid_roundtrip_mixed_cells_80x24() {
        let mut cells = Vec::with_capacity(80 * 24);
        for i in 0..80 * 24 {
            let ch = match i % 5 {
                0 => ' ',
                1 => '⠿',  // braille
                2 => 'ア', // wide char
                3 => '▀',  // half block
                _ => 'z',
            };
            let fg = (i % 3 != 0).then_some(Color::Rgb {
                r: (i % 256) as u8,
                g: 3,
                b: 200,
            });
            let bg = (i % 7 == 0).then_some(Color::AnsiValue(196));
            cells.push(cell(ch, fg, bg));
        }
        let grid = CellGrid {
            cols: 80,
            rows: 24,
            cells,
        };
        let out = roundtrip(&grid);
        assert_eq!(out.cols, 80);
        assert_eq!(out.rows, 24);
        for (got, want) in out.cells.iter().zip(grid.cells.iter()) {
            assert_eq!(got.ch, want.ch);
            // AnsiValue normalizes to its RGB triple.
            match want.fg {
                Some(Color::AnsiValue(n)) => {
                    assert_eq!(got.fg, rgb_tuple(crate::color::ansi256_to_rgb(n)))
                }
                _ => assert_eq!(got.fg, want.fg),
            }
            match want.bg {
                Some(Color::AnsiValue(n)) => {
                    assert_eq!(got.bg, rgb_tuple(crate::color::ansi256_to_rgb(n)))
                }
                _ => assert_eq!(got.bg, want.bg),
            }
        }
    }

    fn rgb_tuple((r, g, b): (u8, u8, u8)) -> Option<Color> {
        Some(Color::Rgb { r, g, b })
    }

    #[test]
    fn grid_roundtrip_rle_boundaries() {
        // Runs of 1, exactly 65535, and 65536 (splits into two runs).
        for len in [1usize, 65535, 65536, 70000] {
            let grid = CellGrid {
                cols: len,
                rows: 1,
                cells: vec![cell('a', rgb(9, 9, 9), None); len],
            };
            let out = roundtrip(&grid);
            assert_eq!(out.cells.len(), len, "len {len}");
            assert!(
                out.cells
                    .iter()
                    .all(|c| c.ch == 'a' && c.fg == rgb(9, 9, 9))
            );
        }
    }

    #[test]
    fn grid_roundtrip_default_cell_run_is_tiny() {
        let grid = CellGrid::new(80, 24);
        // One run: u16 count + flags + char len + ' ' = 5 bytes for 1920 cells.
        assert_eq!(encode_grid(&grid).len(), 5);
    }

    // ---- decode_grid rejection of malformed input ----

    #[test]
    fn decode_rejects_truncated_header() {
        let err = decode_grid(&[0x01, 0x00], 1, 1)
            .err()
            .expect("should reject truncated header");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn decode_rejects_zero_run() {
        let mut data = encode_grid(&CellGrid {
            cols: 1,
            rows: 1,
            cells: vec![cell('a', None, None)],
        });
        data[0] = 0;
        data[1] = 0;
        assert!(decode_grid(&data, 1, 1).is_err());
    }

    #[test]
    fn decode_rejects_bad_char_length() {
        let mut data = encode_grid(&CellGrid {
            cols: 1,
            rows: 1,
            cells: vec![cell('a', None, None)],
        });
        data[3] = 7; // char length out of 1..=4
        assert!(decode_grid(&data, 1, 1).is_err());
    }

    #[test]
    fn decode_rejects_run_overflow() {
        // A run claiming 65535 cells against a 1x1 grid.
        let mut data = encode_grid(&CellGrid {
            cols: 1,
            rows: 1,
            cells: vec![cell('a', None, None)],
        });
        data[0] = 0xFF;
        data[1] = 0xFF;
        assert!(decode_grid(&data, 1, 1).is_err());
    }

    #[test]
    fn decode_rejects_cell_count_mismatch() {
        // Two cells encoded against a 1x1 grid.
        let grid = CellGrid {
            cols: 2,
            rows: 1,
            cells: vec![cell('a', None, None), cell('b', None, None)],
        };
        let data = encode_grid(&grid);
        assert!(decode_grid(&data, 1, 1).is_err());
    }

    #[test]
    fn decode_rejects_truncated_color() {
        let grid = CellGrid {
            cols: 1,
            rows: 1,
            cells: vec![cell('a', rgb(1, 2, 3), None)],
        };
        let mut data = encode_grid(&grid);
        data.pop(); // chop one fg byte
        assert!(decode_grid(&data, 1, 1).is_err());
    }

    #[test]
    fn decode_rejects_multichar_payload_in_one_char_slot() {
        // count=1, flags=0, len=2, payload "ab" — two chars in one slot.
        let crafted = vec![1u8, 0, 0, 2, b'a', b'b'];
        assert!(decode_grid(&crafted, 2, 1).is_err());
    }

    // ---- file-level round-trips ----

    #[test]
    fn save_load_v2_roundtrip() {
        let path = tmp_path("v2-roundtrip.asciianim");
        let mut rec = Recorder::new();
        let g1 = CellGrid {
            cols: 4,
            rows: 2,
            cells: vec![cell('⠋', rgb(10, 20, 30), None); 8],
        };
        let mut g2_cells = vec![cell(' ', None, None); 8];
        g2_cells[3] = cell('Z', None, rgb(5, 6, 7));
        let g2 = CellGrid {
            cols: 4,
            rows: 2,
            cells: g2_cells,
        };
        rec.capture_grid(&g1);
        rec.capture_grid(&g2);
        rec.save(&path).unwrap();

        let player = Player::load(&path).unwrap();
        let Frames::V2 { cols, rows, frames } = player.frames() else {
            panic!("expected v2");
        };
        assert_eq!((*cols, *rows), (4, 2));
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].grid.cells, g1.cells);
        assert_eq!(frames[1].grid.cells, g2.cells);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_writes_v2_header_and_size() {
        let path = tmp_path("v2-header.asciianim");
        let mut rec = Recorder::new();
        rec.capture_grid(&CellGrid::new(80, 24));
        rec.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("ASCIIANIM v2"));
        assert_eq!(lines.next(), Some("SIZE 80 24"));
        assert!(lines.next().unwrap().starts_with("FRAMES 1"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn recorder_fits_resized_grids_to_first_dims() {
        let mut rec = Recorder::new();
        rec.capture_grid(&CellGrid::new(4, 2));
        rec.capture_grid(&CellGrid::new(8, 4)); // resize mid-recording
        let path = tmp_path("v2-resize.asciianim");
        rec.save(&path).unwrap();
        let player = Player::load(&path).unwrap();
        let Frames::V2 { cols, rows, frames } = player.frames() else {
            panic!("expected v2");
        };
        assert_eq!((*cols, *rows), (4, 2));
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].grid.cols, 4);
        assert_eq!(frames[1].grid.rows, 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v1_fixture_still_works() {
        let path = tmp_path("v1-fixture.asciianim");
        let content1 = "\x1b[1;1H\x1b[38;2;255;0;0mHello";
        let content2 = "\x1b[1;1H\x1b[38;2;0;255;0mWorld";
        let text = format!(
            "ASCIIANIM v1\nFRAMES 2\n---\nT 0\n{}\n---\nT 100\n{}\n",
            base64_encode(content1.as_bytes()),
            base64_encode(content2.as_bytes()),
        );
        std::fs::write(&path, text).unwrap();
        let player = Player::load(&path).unwrap();
        let Frames::V1(frames) = player.frames() else {
            panic!("expected v1");
        };
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].content, content1);
        assert_eq!(frames[1].timestamp_ms, 100);
        assert!(!player.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v2_rejects_oversize_size_without_oom() {
        let path = tmp_path("v2-huge.asciianim");
        let text = "ASCIIANIM v2\nSIZE 70000 70000\nFRAMES 0\n";
        std::fs::write(&path, text).unwrap();
        let err = Player::load(&path)
            .err()
            .expect("should reject oversize SIZE");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("out of range"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v2_rejects_missing_size() {
        let path = tmp_path("v2-nosize.asciianim");
        std::fs::write(&path, "ASCIIANIM v2\nFRAMES 0\n").unwrap();
        assert!(Player::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v2_rejects_zero_size() {
        let path = tmp_path("v2-zerosize.asciianim");
        std::fs::write(&path, "ASCIIANIM v2\nSIZE 0 0\nFRAMES 0\n").unwrap();
        assert!(Player::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v2_rejects_frame_count_over_limit() {
        let path = tmp_path("v2-manyframes.asciianim");
        std::fs::write(&path, "ASCIIANIM v2\nSIZE 80 24\nFRAMES 100001\n").unwrap();
        assert!(Player::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v2_rejects_corrupt_frame_body() {
        let path = tmp_path("v2-corrupt.asciianim");
        // Valid header, garbage frame payload.
        std::fs::write(
            &path,
            format!(
                "ASCIIANIM v2\nSIZE 80 24\nFRAMES 1\n---\nT 0\n{}\n",
                base64_encode(&[0xFF, 0xFF, 0x01, 0x09, b'x'])
            ),
        )
        .unwrap();
        assert!(Player::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_v2_tolerates_size_at_limits() {
        let path = tmp_path("v2-maxsize.asciianim");
        std::fs::write(&path, "ASCIIANIM v2\nSIZE 1000 500\nFRAMES 0\n").unwrap();
        let player = Player::load(&path).unwrap();
        assert!(player.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
