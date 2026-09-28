//! Hand-written GIF89a encoder with LZW compression and ANSI virtual terminal decoder.
//!
//! No external crate dependencies. Converts `.asciianim` frame data into an animated GIF
//! by decoding ANSI escape sequences, quantizing truecolor to a 6x7x6 palette (252 colors
//! + 4 reserved), and writing GIF frames with variable-width LZW.

use std::io::Write;

use crate::color::ansi256_to_rgb;
use crate::render::cell::CellGrid;

/// Upper bounds on recording-derived dimensions accepted by the GIF exporters.
/// GIF dimension fields are u16; these tighter limits also cap the per-frame
/// pixel buffers, so a crafted recording cannot request gigabytes of cells.
pub const MAX_GIF_COLS: usize = 2048;
pub const MAX_GIF_ROWS: usize = 2048;

fn invalid_gif_dims(cols: usize, rows: usize) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("GIF dimensions {cols}x{rows} out of range (max {MAX_GIF_COLS}x{MAX_GIF_ROWS})"),
    )
}

/// Validate recording-derived dimensions: nonzero, within the GIF size budget,
/// and exactly representable as u16.
fn validate_gif_dims(cols: usize, rows: usize) -> std::io::Result<(u16, u16, usize)> {
    if cols == 0 || rows == 0 {
        return Err(invalid_gif_dims(cols, rows));
    }
    let width = u16::try_from(cols).map_err(|_| invalid_gif_dims(cols, rows))?;
    let height = u16::try_from(rows).map_err(|_| invalid_gif_dims(cols, rows))?;
    let pixel_count = cols
        .checked_mul(rows)
        .ok_or_else(|| invalid_gif_dims(cols, rows))?;
    if pixel_count > MAX_GIF_COLS * MAX_GIF_ROWS {
        return Err(invalid_gif_dims(cols, rows));
    }
    Ok((width, height, pixel_count))
}

// ---------------------------------------------------------------------------
// Virtual terminal — decodes ANSI sequences produced by termflix's renderer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Cell {
    pub ch: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: b' ',
            r: 0,
            g: 0,
            b: 0,
        }
    }
}

pub struct VirtualTerminal {
    cells: Vec<Cell>,
    cols: usize,
    rows: usize,
    cursor_row: usize,
    cursor_col: usize,
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
}

impl VirtualTerminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        VirtualTerminal {
            cells: vec![Cell::default(); cols * rows],
            cols,
            rows,
            cursor_row: 0,
            cursor_col: 0,
            fg_r: 0,
            fg_g: 0,
            fg_b: 0,
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    pub fn process(&mut self, data: &str) {
        let bytes = data.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
                // Parse CSI sequence
                i += 2;
                let start = i;
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b';' || bytes[i] == b'?')
                {
                    i += 1;
                }
                if i >= len {
                    break;
                }
                let params_str = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                let cmd = bytes[i];
                i += 1;

                match cmd {
                    b'H' => {
                        // CUP — cursor position
                        let parts: Vec<&str> = params_str.split(';').collect();
                        let row = parts
                            .first()
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1)
                            .saturating_sub(1);
                        let col = parts
                            .get(1)
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1)
                            .saturating_sub(1);
                        self.cursor_row = row.min(self.rows - 1);
                        self.cursor_col = col.min(self.cols - 1);
                    }
                    b'm' => {
                        // SGR — select graphic rendition
                        if params_str.is_empty() || params_str == "0" {
                            // Reset
                            self.fg_r = 0;
                            self.fg_g = 0;
                            self.fg_b = 0;
                        } else {
                            let nums: Vec<u32> = params_str
                                .split(';')
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            Self::parse_sgr(&nums, &mut self.fg_r, &mut self.fg_g, &mut self.fg_b);
                        }
                    }
                    b'h' | b'l' => {
                        // Mode set/reset — ignore (BSU sync markers, etc.)
                    }
                    b'K' => {
                        // Erase to end of line — ignore
                    }
                    _ => {}
                }
            } else {
                // Printable character — handle both ASCII and multi-byte UTF-8
                let ch = bytes[i];
                let (is_printable, advance) = if ch.is_ascii() {
                    (!ch.is_ascii_control(), 1)
                } else {
                    // Decode UTF-8 lead byte to find sequence length
                    let seq_len = if ch & 0xE0 == 0xC0 {
                        2
                    } else if ch & 0xF0 == 0xE0 {
                        3
                    } else if ch & 0xF8 == 0xF0 {
                        4
                    } else {
                        1 // invalid lead byte, skip
                    };
                    (seq_len > 1, seq_len)
                };

                if is_printable {
                    // Store ASCII chars directly; use sentinel b'#' for non-ASCII
                    let stored_ch = if ch.is_ascii() { ch } else { b'#' };
                    if self.cursor_row < self.rows && self.cursor_col < self.cols {
                        let idx = self.cursor_row * self.cols + self.cursor_col;
                        self.cells[idx] = Cell {
                            ch: stored_ch,
                            r: self.fg_r,
                            g: self.fg_g,
                            b: self.fg_b,
                        };
                    }
                    self.cursor_col += 1;
                    if self.cursor_col >= self.cols {
                        self.cursor_col = 0;
                        if self.cursor_row + 1 < self.rows {
                            self.cursor_row += 1;
                        }
                    }
                }
                i += advance;
            }
        }
    }

    fn parse_sgr(nums: &[u32], r: &mut u8, g: &mut u8, b: &mut u8) {
        let mut i = 0;
        while i < nums.len() {
            match nums[i] {
                0 => {
                    *r = 0;
                    *g = 0;
                    *b = 0;
                }
                38 if i + 1 < nums.len() => {
                    if nums[i + 1] == 2 && i + 4 < nums.len() {
                        // Truecolor: 38;2;r;g;b
                        *r = nums[i + 2] as u8;
                        *g = nums[i + 3] as u8;
                        *b = nums[i + 4] as u8;
                        i += 4;
                    } else if nums[i + 1] == 5 && i + 2 < nums.len() {
                        // 256-color: 38;5;N
                        let (cr, cg, cb) = ansi256_to_rgb(nums[i + 2] as u8);
                        *r = cr;
                        *g = cg;
                        *b = cb;
                        i += 2;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Color quantization — 6x7x6 uniform palette (252 entries + 4 reserved)
// ---------------------------------------------------------------------------

const PALETTE_R_LEVELS: usize = 6;
const PALETTE_G_LEVELS: usize = 7;
const PALETTE_B_LEVELS: usize = 6;
const PALETTE_COLOR_COUNT: usize = PALETTE_R_LEVELS * PALETTE_G_LEVELS * PALETTE_B_LEVELS;
// Total palette: 252 + 4 reserved = 256 (fits GIF 8-bit global color table)
const PALETTE_SIZE: usize = PALETTE_COLOR_COUNT + 4;

struct Palette {
    entries: [(u8, u8, u8); PALETTE_SIZE],
}

impl Palette {
    fn new() -> Self {
        let mut entries = [(0u8, 0u8, 0u8); PALETTE_SIZE];

        // 6x7x6 uniform cube
        let mut idx = 0;
        for ri in 0..PALETTE_R_LEVELS {
            let r = if PALETTE_R_LEVELS > 1 {
                (ri * 255 / (PALETTE_R_LEVELS - 1)) as u8
            } else {
                0
            };
            for gi in 0..PALETTE_G_LEVELS {
                let g = if PALETTE_G_LEVELS > 1 {
                    (gi * 255 / (PALETTE_G_LEVELS - 1)) as u8
                } else {
                    0
                };
                for bi in 0..PALETTE_B_LEVELS {
                    let b = if PALETTE_B_LEVELS > 1 {
                        (bi * 255 / (PALETTE_B_LEVELS - 1)) as u8
                    } else {
                        0
                    };
                    entries[idx] = (r, g, b);
                    idx += 1;
                }
            }
        }

        // 4 reserved safety colors: black, dark gray, light gray, white
        entries[252] = (0, 0, 0);
        entries[253] = (64, 64, 64);
        entries[254] = (192, 192, 192);
        entries[255] = (255, 255, 255);

        Palette { entries }
    }

    fn find_nearest(&self, r: u8, g: u8, b: u8) -> u8 {
        let mut best_idx: u8 = 0;
        let mut best_dist: u32 = u32::MAX;
        for (i, &(pr, pg, pb)) in self.entries.iter().enumerate() {
            // Absolute channel differences; the i32->u32 cast on a negative
            // difference would wrap to ~4e9 and overflow when squared.
            let dr = (r as i32 - pr as i32).unsigned_abs();
            let dg = (g as i32 - pg as i32).unsigned_abs();
            let db = (b as i32 - pb as i32).unsigned_abs();
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best_idx = i as u8;
                if dist == 0 {
                    break;
                }
            }
        }
        best_idx
    }
}

// ---------------------------------------------------------------------------
// LZW compressor — variable-width, LSB-first packing
// ---------------------------------------------------------------------------

struct BitPacker {
    buf: Vec<u8>,
    pending: u32,
    pending_bits: u8,
}

impl BitPacker {
    fn new() -> Self {
        BitPacker {
            buf: Vec::new(),
            pending: 0,
            pending_bits: 0,
        }
    }

    fn write_bits(&mut self, code: u32, width: u8) {
        self.pending |= code << self.pending_bits;
        self.pending_bits += width;
        while self.pending_bits >= 8 {
            self.buf.push((self.pending & 0xFF) as u8);
            self.pending >>= 8;
            self.pending_bits -= 8;
        }
    }

    fn flush(&mut self) {
        if self.pending_bits > 0 {
            self.buf.push((self.pending & 0xFF) as u8);
            self.pending = 0;
            self.pending_bits = 0;
        }
    }
}

#[derive(Clone)]
struct LzwEntry {
    prefix: u16,
    byte: u8,
}

struct LzwEncoder {
    min_code_size: u8,
    clear_code: u16,
    eoi_code: u16,
    next_code: u16,
    max_code: u16,
    code_width: u8,
    table: Vec<Option<LzwEntry>>,
    packer: BitPacker,
}

impl LzwEncoder {
    fn new(min_code_size: u8) -> Self {
        let clear_code = 1u16 << min_code_size;
        let eoi_code = clear_code + 1;
        let initial_width = min_code_size + 1;
        let mut table = Vec::new();
        table.resize(4096, None);
        LzwEncoder {
            min_code_size,
            clear_code,
            eoi_code,
            next_code: eoi_code + 1,
            max_code: (1u16 << initial_width as u16) - 1,
            code_width: initial_width,
            table,
            packer: BitPacker::new(),
        }
    }

    fn reset(&mut self) {
        self.next_code = self.eoi_code + 1;
        self.code_width = self.min_code_size + 1;
        self.max_code = (1u16 << self.code_width as u16) - 1;
    }

    fn encode(&mut self, indices: &[u8]) -> Vec<u8> {
        self.packer.buf.clear();
        self.packer.pending = 0;
        self.packer.pending_bits = 0;
        self.reset();
        self.table.fill(None);

        // Emit clear code
        self.packer
            .write_bits(self.clear_code as u32, self.code_width);

        if indices.is_empty() {
            self.packer
                .write_bits(self.eoi_code as u32, self.code_width);
            self.packer.flush();
            return self.packer.buf.clone();
        }

        let mut current = indices[0] as u16;

        for &byte in &indices[1..] {
            // Look up (current_prefix, byte) in table
            let mut found = None;
            for code in (self.eoi_code as usize + 1)..self.next_code as usize {
                if let Some(ref entry) = self.table[code]
                    && entry.prefix == current
                    && entry.byte == byte
                {
                    found = Some(code as u16);
                    break;
                }
            }

            if let Some(code) = found {
                current = code;
            } else {
                // Emit current prefix code
                self.packer.write_bits(current as u32, self.code_width);

                // Add new entry if table not full
                if self.next_code < 4096 {
                    self.table[self.next_code as usize] = Some(LzwEntry {
                        prefix: current,
                        byte,
                    });
                    self.next_code += 1;

                    // Bump width when next_code would no longer fit. Standard
                    // GIF LZW (giflib/Pillow) bumps at `next_code > 1<<width`,
                    // i.e., one step LATER than `> max_code`. The decoder's
                    // add lags the encoder's by one read, so bumping at
                    // `> max_code` corrupts everything past the first bump.
                    if self.next_code > (1u16 << self.code_width as u16) && self.code_width < 12 {
                        self.code_width += 1;
                        self.max_code = (1u16 << self.code_width as u16) - 1;
                    }
                } else {
                    // Table full — emit clear and reset
                    self.packer
                        .write_bits(self.clear_code as u32, self.code_width);
                    self.reset();
                    self.table.fill(None);
                }

                current = byte as u16;
            }
        }

        // Emit final code
        self.packer.write_bits(current as u32, self.code_width);
        // Emit EOI
        self.packer
            .write_bits(self.eoi_code as u32, self.code_width);
        self.packer.flush();

        self.packer.buf.clone()
    }
}

// ---------------------------------------------------------------------------
// GIF89a writer
// ---------------------------------------------------------------------------

/// Streaming GIF89a writer shared by `export_gif` and `export_gif_pixels`.
struct GifWriter<'a, W: Write> {
    writer: &'a mut W,
    encoder: LzwEncoder,
}

impl<'a, W: Write> GifWriter<'a, W> {
    /// Writes the GIF89a header, Logical Screen Descriptor, Global Color
    /// Table, and the NETSCAPE2.0 infinite-loop extension.
    fn new(writer: &'a mut W, width: u16, height: u16) -> std::io::Result<Self> {
        let palette = Palette::new();

        // Build palette bytes — GIF requires power-of-2 table size, so round up to 256
        let mut pal_bytes = [0u8; 768]; // 256 * 3
        for (i, &(r, g, b)) in palette.entries.iter().enumerate() {
            pal_bytes[i * 3] = r;
            pal_bytes[i * 3 + 1] = g;
            pal_bytes[i * 3 + 2] = b;
        }

        writer.write_all(b"GIF89a")?;

        // Logical Screen Descriptor: width/height (LE16), packed byte, bg
        // color, pixel aspect ratio. Packed = GCT flag | color resolution
        // (8-1=7) | sort=0 | size=7 (2^(7+1)=256).
        let packed = 0x80 | 0x07;
        writer.write_all(&width.to_le_bytes())?;
        writer.write_all(&height.to_le_bytes())?;
        writer.write_all(&[packed, 0, 0])?;

        // Global Color Table (256 * 3 bytes)
        writer.write_all(&pal_bytes)?;

        // NETSCAPE2.0 Application Extension (loop forever)
        writer.write_all(&[
            0x21, // Extension introducer
            0xFF, // Application extension label
            11,   // Block size
        ])?;
        writer.write_all(b"NETSCAPE2.0")?;
        writer.write_all(&[
            3, // Sub-block size
            1, // Sub-block ID for looping
            0, 0, // Loop count (0 = infinite)
            0, // Block terminator
        ])?;

        Ok(GifWriter {
            writer,
            encoder: LzwEncoder::new(8), // min code size = 8 for 256-color palette
        })
    }

    /// Writes one frame: Graphic Control Extension carrying `delay_cs`,
    /// Image Descriptor, and `indices` LZW-compressed into sub-blocks.
    fn write_frame(
        &mut self,
        indices: &[u8],
        delay_cs: u16,
        width: u16,
        height: u16,
    ) -> std::io::Result<()> {
        // Graphic Control Extension
        self.writer.write_all(&[
            0x21, // Extension introducer
            0xF9, // Graphic Control label
            4,    // Block size
            0x00, // Packed: dispose=0, no user input, no transparent
        ])?;
        self.writer.write_all(&delay_cs.to_le_bytes())?;
        self.writer.write_all(&[
            0, // Transparent color index (unused)
            0, // Block terminator
        ])?;

        // Image Descriptor
        self.writer.write_all(&[
            0x2C, // Image separator
        ])?;
        self.writer.write_all(&0u16.to_le_bytes())?; // Left
        self.writer.write_all(&0u16.to_le_bytes())?; // Top
        self.writer.write_all(&width.to_le_bytes())?; // Width
        self.writer.write_all(&height.to_le_bytes())?; // Height
        self.writer.write_all(&[0x00])?; // Packed: no local color table

        // LZW Minimum Code Size — required between Image Descriptor and the
        // sub-block stream. For an 8-bit palette this is 8.
        self.writer.write_all(&[8])?;

        // LZW-compressed image data
        let compressed = self.encoder.encode(indices);

        // Write as sub-blocks (max 255 bytes each)
        let mut pos = 0;
        while pos < compressed.len() {
            let chunk_len = (compressed.len() - pos).min(255);
            self.writer.write_all(&[chunk_len as u8])?;
            self.writer.write_all(&compressed[pos..pos + chunk_len])?;
            pos += chunk_len;
        }
        self.writer.write_all(&[0])?; // Block terminator
        Ok(())
    }

    /// Writes the GIF trailer and flushes.
    fn finish(self) -> std::io::Result<()> {
        self.writer.write_all(&[0x3B])?;
        self.writer.flush()
    }
}

/// Export recorded frames as an animated GIF.
///
/// `term_cols` and `term_rows` are the terminal dimensions (in character cells).
/// Each cell maps to one GIF pixel. The image width = term_cols, height = term_rows.
pub fn export_gif<W: Write>(
    writer: &mut W,
    frames: &[crate::record::Frame],
    term_cols: usize,
    term_rows: usize,
) -> std::io::Result<()> {
    let palette = Palette::new();
    let (width, height, pixel_count) = validate_gif_dims(term_cols, term_rows)?;

    let mut gif = GifWriter::new(writer, width, height)?;
    let mut prev_indices: Vec<u8> = Vec::new();
    let mut pending_delay_cs: u16 = 0;

    for (fi, frame) in frames.iter().enumerate() {
        // Decode ANSI into virtual terminal
        let mut vt = VirtualTerminal::new(term_cols, term_rows);
        vt.process(&frame.content);

        // Build index array
        let mut indices = vec![0u8; pixel_count];
        for row in 0..term_rows {
            for col in 0..term_cols {
                let cell = vt.cell(row, col);
                let idx = if cell.ch == b' ' {
                    0 // Background/black for empty cells
                } else {
                    palette.find_nearest(cell.r, cell.g, cell.b)
                };
                indices[row * term_cols + col] = idx;
            }
        }

        // Frame deduplication: skip identical consecutive frames, accumulate delay.
        // The delay for a written frame includes any time accumulated from the
        // skipped duplicates before it.
        let delay_cs = if fi + 1 < frames.len() {
            let delta_ms = frames[fi + 1]
                .timestamp_ms
                .saturating_sub(frame.timestamp_ms);
            (delta_ms / 10).clamp(2, 65535) as u16
        } else {
            2 // Minimum 2 centiseconds for last frame
        };

        if indices == prev_indices {
            pending_delay_cs = pending_delay_cs.saturating_add(delay_cs);
            continue;
        }

        let total_delay = pending_delay_cs.saturating_add(delay_cs);
        pending_delay_cs = 0;

        gif.write_frame(&indices, total_delay, width, height)?;
        prev_indices = indices;
    }

    gif.finish()
}

// ---------------------------------------------------------------------------
// Pixel-based GIF export — bypasses VirtualTerminal entirely
// ---------------------------------------------------------------------------

/// One frame of RGB pixel data plus its capture timestamp.
///
/// `pixels` is row-major, length must equal `width * height` of the
/// dimensions passed to [`export_gif_pixels`].
pub struct PixelFrame {
    pub timestamp_ms: u64,
    pub pixels: Vec<(u8, u8, u8)>,
}

/// Convert a v2 recording frame's cell grid into RGB pixels for
/// [`export_gif_pixels`]: empty cells are black, everything else takes its
/// foreground color. Mirrors what the v1 `VirtualTerminal` path produces
/// after parsing a frame.
pub fn render_cells_to_pixels(grid: &CellGrid) -> Vec<(u8, u8, u8)> {
    grid.cells
        .iter()
        .map(|c| {
            if c.ch == ' ' {
                (0, 0, 0)
            } else {
                c.fg.map_or((0, 0, 0), crate::color::color_to_rgb)
            }
        })
        .collect()
}

/// Export RGB pixel frames as an animated GIF.
///
/// `width` / `height` are the native pixel dimensions of each frame.
/// `scale` upscales each pixel to a `scale x scale` block via nearest-neighbor,
/// so the resulting GIF is `width * scale` by `height * scale`.
pub fn export_gif_pixels<W: Write>(
    writer: &mut W,
    frames: &[PixelFrame],
    width: usize,
    height: usize,
    scale: usize,
) -> std::io::Result<()> {
    assert!(scale >= 1, "scale must be >= 1");
    let palette = Palette::new();
    let out_w = u16::try_from(
        width
            .checked_mul(scale)
            .ok_or_else(|| invalid_gif_dims(width, height))?,
    )
    .map_err(|_| invalid_gif_dims(width, height))?;
    let out_h = u16::try_from(
        height
            .checked_mul(scale)
            .ok_or_else(|| invalid_gif_dims(width, height))?,
    )
    .map_err(|_| invalid_gif_dims(width, height))?;
    let native_count = width
        .checked_mul(height)
        .ok_or_else(|| invalid_gif_dims(width, height))?;
    let scaled_count = native_count
        .checked_mul(scale)
        .and_then(|c| c.checked_mul(scale))
        .ok_or_else(|| invalid_gif_dims(width, height))?;

    let mut gif = GifWriter::new(writer, out_w, out_h)?;
    let mut prev_native: Vec<u8> = Vec::new();
    let mut pending_delay_cs: u16 = 0;

    for (fi, frame) in frames.iter().enumerate() {
        if frame.pixels.len() != native_count {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "frame {} has {} pixels, expected {}",
                    fi,
                    frame.pixels.len(),
                    native_count
                ),
            ));
        }

        // Compute palette indices at native resolution (cheap dedup key).
        let mut native = vec![0u8; native_count];
        for (i, &(r, g, b)) in frame.pixels.iter().enumerate() {
            native[i] = palette.find_nearest(r, g, b);
        }

        let delay_cs = if fi + 1 < frames.len() {
            let delta_ms = frames[fi + 1]
                .timestamp_ms
                .saturating_sub(frame.timestamp_ms);
            (delta_ms / 10).clamp(2, 65535) as u16
        } else {
            2
        };

        if native == prev_native {
            pending_delay_cs = pending_delay_cs.saturating_add(delay_cs);
            continue;
        }

        let total_delay = pending_delay_cs.saturating_add(delay_cs);
        pending_delay_cs = 0;

        // Nearest-neighbor upscale to scaled_count.
        let scaled = if scale == 1 {
            native.clone()
        } else {
            let mut out = vec![0u8; scaled_count];
            let stride = width * scale;
            for ny in 0..height {
                for nx in 0..width {
                    let idx = native[ny * width + nx];
                    let base_y = ny * scale;
                    let base_x = nx * scale;
                    for dy in 0..scale {
                        let row_start = (base_y + dy) * stride + base_x;
                        for dx in 0..scale {
                            out[row_start + dx] = idx;
                        }
                    }
                }
            }
            out
        };

        gif.write_frame(&scaled, total_delay, out_w, out_h)?;
        prev_native = native;
    }

    gif.finish()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi256_to_rgb_black() {
        assert_eq!(ansi256_to_rgb(0), (0, 0, 0));
    }

    #[test]
    fn test_ansi256_to_rgb_white() {
        assert_eq!(ansi256_to_rgb(15), (255, 255, 255));
    }

    #[test]
    fn test_ansi256_to_rgb_color_cube() {
        // Index 196 = 5*36+0*6+0 => r=255, g=0, b=0
        let (r, g, b) = ansi256_to_rgb(196);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_ansi256_to_rgb_grayscale() {
        let (r, g, b) = ansi256_to_rgb(232);
        assert_eq!(r, 8);
        assert_eq!(g, r);
        assert_eq!(b, r);
    }

    #[test]
    fn test_palette_nearest_black() {
        let palette = Palette::new();
        assert_eq!(palette.find_nearest(0, 0, 0), 0);
    }

    #[test]
    fn test_palette_nearest_white() {
        let palette = Palette::new();
        let idx = palette.find_nearest(255, 255, 255);
        let (r, g, b) = palette.entries[idx as usize];
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_palette_nearest_off_palette_color_no_overflow() {
        // Off-palette color whose channels sit below many palette entries:
        // the distance loop must handle negative channel differences.
        let palette = Palette::new();
        let _ = palette.find_nearest(10, 200, 30);
    }

    #[test]
    fn test_palette_nearest_prefers_closest_entry() {
        let palette = Palette::new();
        // (250, 250, 250) is 3x5 away from white (255,255,255) and much
        // farther from gray ramp entries; nearest must resolve to white.
        let idx = palette.find_nearest(250, 250, 250);
        let (r, g, b) = palette.entries[idx as usize];
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_virtual_terminal_cursor_position() {
        let mut vt = VirtualTerminal::new(10, 5);
        vt.process("\x1b[2;5HX");
        assert_eq!(vt.cell(1, 4).ch, b'X');
    }

    #[test]
    fn test_virtual_terminal_truecolor() {
        let mut vt = VirtualTerminal::new(10, 5);
        vt.process("\x1b[38;2;255;0;128mA");
        let cell = vt.cell(0, 0);
        assert_eq!(cell.ch, b'A');
        assert_eq!(cell.r, 255);
        assert_eq!(cell.g, 0);
        assert_eq!(cell.b, 128);
    }

    #[test]
    fn test_virtual_terminal_256color() {
        let mut vt = VirtualTerminal::new(10, 5);
        vt.process("\x1b[38;5;196mB");
        let cell = vt.cell(0, 0);
        assert_eq!(cell.ch, b'B');
        let (r, g, b) = ansi256_to_rgb(196);
        assert_eq!(cell.r, r);
        assert_eq!(cell.g, g);
        assert_eq!(cell.b, b);
    }

    #[test]
    fn test_virtual_terminal_reset() {
        let mut vt = VirtualTerminal::new(10, 5);
        vt.process("\x1b[38;2;255;0;0mA\x1b[mB");
        let cell_a = vt.cell(0, 0);
        assert_eq!(cell_a.r, 255);
        let cell_b = vt.cell(0, 1);
        assert_eq!(cell_b.r, 0);
    }

    #[test]
    fn test_virtual_terminal_ignores_bsu_markers() {
        let mut vt = VirtualTerminal::new(10, 5);
        // BSU sync markers should not affect output
        vt.process("\x1b[?2026hHello\x1b[?2026l");
        assert_eq!(vt.cell(0, 0).ch, b'H');
        assert_eq!(vt.cell(0, 4).ch, b'o');
    }

    #[test]
    fn test_lzw_roundtrip() {
        // Encode a simple pattern and verify output is non-empty and well-formed
        let mut encoder = LzwEncoder::new(8);
        let indices: Vec<u8> = (0..100).map(|i| (i % 7) as u8).collect();
        let compressed = encoder.encode(&indices);
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_lzw_empty() {
        let mut encoder = LzwEncoder::new(8);
        let compressed = encoder.encode(&[]);
        assert!(!compressed.is_empty()); // Should still have clear + EOI codes
    }

    /// Reference GIF-LZW decoder used to verify the encoder round-trips.
    /// Returns the decoded bytes.
    fn lzw_decode(data: &[u8], min_code_size: u8) -> Vec<u8> {
        let clear_code = 1u16 << min_code_size;
        let eoi_code = clear_code + 1;
        let init_width = min_code_size + 1;

        let mut bit_pos = 0usize;
        let read_code = |bit_pos: &mut usize, width: u8| -> Option<u16> {
            let total_bits = data.len() * 8;
            if *bit_pos + width as usize > total_bits {
                return None;
            }
            let mut code = 0u32;
            for i in 0..width as usize {
                let p = *bit_pos + i;
                let bit = (data[p / 8] >> (p % 8)) & 1;
                code |= (bit as u32) << i;
            }
            *bit_pos += width as usize;
            Some(code as u16)
        };

        let mut dict: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8]).collect();
        dict.push(Vec::new()); // clear
        dict.push(Vec::new()); // eoi

        let mut next_code = (eoi_code + 1) as usize;
        let mut code_width = init_width;

        let mut output = Vec::new();
        let mut prev: Option<u16> = None;

        while let Some(code) = read_code(&mut bit_pos, code_width) {
            if code == clear_code {
                dict.truncate(258);
                next_code = (eoi_code + 1) as usize;
                code_width = init_width;
                prev = None;
                continue;
            }
            if code == eoi_code {
                break;
            }

            let entry: Vec<u8> = if (code as usize) < dict.len() {
                dict[code as usize].clone()
            } else if let Some(p) = prev {
                if (p as usize) >= dict.len() {
                    panic!(
                        "decoder diverged: code={}, prev={}, dict.len={}, next_code={}, code_width={}",
                        code,
                        p,
                        dict.len(),
                        next_code,
                        code_width
                    );
                }
                if code as usize != next_code {
                    panic!(
                        "code {} > dict.len()={} but != next_code={}: bit-stream out of sync",
                        code,
                        dict.len(),
                        next_code
                    );
                }
                let mut e = dict[p as usize].clone();
                e.push(dict[p as usize][0]);
                e
            } else {
                panic!("first code was outside dict and no prev: {}", code);
            };

            output.extend_from_slice(&entry);

            if let Some(p) = prev
                && next_code < 4096
            {
                let mut new_entry = dict[p as usize].clone();
                new_entry.push(entry[0]);
                if next_code < dict.len() {
                    dict[next_code] = new_entry;
                } else {
                    dict.push(new_entry);
                }
                next_code += 1;
                // Decoder bumps one step earlier than encoder because its
                // add lags by one read: encoder uses `> 1<<width`, decoder
                // uses `>= 1<<width`.
                if next_code as u16 >= (1u16 << code_width) && code_width < 12 {
                    code_width += 1;
                }
            }

            prev = Some(code);
        }

        output
    }

    #[test]
    fn test_lzw_roundtrip_short() {
        let mut encoder = LzwEncoder::new(8);
        let indices: Vec<u8> = (0..100).map(|i| (i % 7) as u8).collect();
        let compressed = encoder.encode(&indices);
        let decoded = lzw_decode(&compressed, 8);
        assert_eq!(decoded, indices);
    }

    #[test]
    fn test_lzw_roundtrip_pseudo_random() {
        // Forces width-bumps without dict-fill; this is the common case
        // for upscaled GIF frames and the symptom the user reported.
        let mut state = 1u32;
        let indices: Vec<u8> = (0..20000)
            .map(|_| {
                state = state.wrapping_mul(1103515245).wrapping_add(12345);
                ((state >> 16) & 0xFF) as u8
            })
            .collect();
        let mut encoder = LzwEncoder::new(8);
        let compressed = encoder.encode(&indices);
        let decoded = lzw_decode(&compressed, 8);
        assert_eq!(decoded.len(), indices.len(), "decoded length mismatch");
        assert_eq!(decoded, indices, "decoded bytes mismatch");
    }

    #[test]
    fn test_lzw_roundtrip_repeats_until_dict_full() {
        // Highly compressible — exercises the dict-fill / reset path.
        let mut indices: Vec<u8> = Vec::new();
        for chunk in 0..200 {
            for i in 0..100u8 {
                indices.push(((chunk + i as usize) % 256) as u8);
            }
        }
        let mut encoder = LzwEncoder::new(8);
        let compressed = encoder.encode(&indices);
        let decoded = lzw_decode(&compressed, 8);
        assert_eq!(decoded, indices);
    }

    #[test]
    fn test_export_gif_single_frame() {
        let frames = vec![crate::record::Frame {
            timestamp_ms: 0,
            content: "Hello".to_string(),
        }];
        let mut buf = Vec::new();
        let result = export_gif(&mut buf, &frames, 10, 5);
        assert!(result.is_ok());
        // Check GIF header
        assert_eq!(&buf[0..6], b"GIF89a");
        // Check trailer
        assert_eq!(*buf.last().unwrap(), 0x3B);
        // Should be at least header + LSD + GCT + extension + frame + trailer
        assert!(buf.len() > 800); // 768 bytes for GCT alone
    }

    #[test]
    fn test_export_gif_dedup_identical_frames() {
        let content = "\x1b[1;1HA";
        let frames = vec![
            crate::record::Frame {
                timestamp_ms: 0,
                content: content.to_string(),
            },
            crate::record::Frame {
                timestamp_ms: 100,
                content: content.to_string(),
            },
            crate::record::Frame {
                timestamp_ms: 200,
                content: content.to_string(),
            },
        ];
        let mut buf = Vec::new();
        export_gif(&mut buf, &frames, 10, 5).unwrap();
        // Should only contain 1 image descriptor (3 identical frames -> 1 written)
        // Count image separators (0x2C)
        let image_count = buf.iter().filter(|&&b| b == 0x2C).count();
        assert_eq!(image_count, 1);
    }
}

#[cfg(test)]
mod sec001_tests {
    use super::*;

    #[test]
    fn render_cells_to_pixels_maps_space_to_black_and_fg_to_rgb() {
        use crate::render::cell::{Cell, CellGrid};
        let grid = CellGrid {
            cols: 3,
            rows: 1,
            cells: vec![
                Cell {
                    ch: ' ',
                    fg: Some(crossterm::style::Color::Rgb {
                        r: 90,
                        g: 90,
                        b: 90,
                    }),
                    bg: None,
                },
                Cell {
                    ch: '▀',
                    fg: Some(crossterm::style::Color::Rgb {
                        r: 10,
                        g: 20,
                        b: 30,
                    }),
                    bg: None,
                },
                Cell {
                    ch: '▀',
                    fg: None,
                    bg: None,
                },
            ],
        };
        assert_eq!(
            render_cells_to_pixels(&grid),
            vec![(0, 0, 0), (10, 20, 30), (0, 0, 0)]
        );
    }

    #[test]
    fn export_gif_rejects_oversize_dims() {
        let mut buf = Vec::new();
        let frames = [crate::record::Frame {
            timestamp_ms: 0,
            content: String::new(),
        }];
        let err = export_gif(&mut buf, &frames, 70_000, 70_000).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn export_gif_rejects_zero_dims() {
        let mut buf = Vec::new();
        let frames = [crate::record::Frame {
            timestamp_ms: 0,
            content: String::new(),
        }];
        let err = export_gif(&mut buf, &frames, 0, 0).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn export_gif_accepts_max_dims() {
        let mut buf = Vec::new();
        let frames = [crate::record::Frame {
            timestamp_ms: 0,
            content: String::new(),
        }];
        export_gif(&mut buf, &frames, MAX_GIF_COLS, MAX_GIF_ROWS).unwrap();
        assert!(buf.starts_with(b"GIF89a"));
    }
}
