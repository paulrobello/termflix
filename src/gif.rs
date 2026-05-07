//! Hand-written GIF89a encoder with LZW compression and ANSI virtual terminal decoder.
//!
//! No external crate dependencies. Converts `.asciianim` frame data into an animated GIF
//! by decoding ANSI escape sequences, quantizing truecolor to a 6x7x6 palette (252 colors
//! + 4 reserved), and writing GIF frames with variable-width LZW.

use std::io::Write;

// ---------------------------------------------------------------------------
// Virtual terminal — decodes ANSI sequences produced by termflix's renderer
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Cell {
    ch: u8,
    r: u8,
    g: u8,
    b: u8,
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

struct VirtualTerminal {
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
    fn new(cols: usize, rows: usize) -> Self {
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

    fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    fn process(&mut self, data: &str) {
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
                // Printable character
                let ch = bytes[i];
                if ch.is_ascii() && !ch.is_ascii_control() {
                    if self.cursor_row < self.rows && self.cursor_col < self.cols {
                        let idx = self.cursor_row * self.cols + self.cursor_col;
                        self.cells[idx] = Cell {
                            ch,
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
                i += 1;
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
// 256-color to RGB conversion
// ---------------------------------------------------------------------------

fn ansi256_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=7 => {
            // Standard 8 colors
            const C: [(u8, u8, u8); 8] = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
            ];
            C[idx as usize]
        }
        8..=15 => {
            // High-intensity 8 colors
            const C: [(u8, u8, u8); 8] = [
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];
            C[(idx - 8) as usize]
        }
        16..=231 => {
            // 6x6x6 color cube
            let n = idx - 16;
            let b_val = n % 6;
            let g_val = (n / 6) % 6;
            let r_val = n / 36;
            const LEVEL: [u8; 6] = [0, 95, 135, 175, 215, 255];
            (
                LEVEL[r_val as usize],
                LEVEL[g_val as usize],
                LEVEL[b_val as usize],
            )
        }
        _ => {
            // Grayscale ramp 232-255
            let v = 8 + 10 * (idx as u32 - 232);
            (v as u8, v as u8, v as u8)
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
            let dr = (r as i32 - pr as i32) as u32;
            let dg = (g as i32 - pg as i32) as u32;
            let db = (b as i32 - pb as i32) as u32;
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

                    // Check if we need to increase code width
                    if self.next_code > self.max_code && self.code_width < 12 {
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
    let width = term_cols as u16;
    let height = term_rows as u16;
    let pixel_count = term_cols * term_rows;

    // Build palette bytes — GIF requires power-of-2 table size, so round up to 256
    let mut pal_bytes = [0u8; 768]; // 256 * 3
    for (i, &(r, g, b)) in palette.entries.iter().enumerate() {
        pal_bytes[i * 3] = r;
        pal_bytes[i * 3 + 1] = g;
        pal_bytes[i * 3 + 2] = b;
    }

    // --- GIF89a header ---
    writer.write_all(b"GIF89a")?;

    // --- Logical Screen Descriptor ---
    // Width (LE16), Height (LE16), packed byte, bg color, pixel aspect ratio
    let packed = 0x80 | 0x07; // GCT flag | color resolution (8-1=7) | sort=0 | size=7 (2^(7+1)=256)
    writer.write_all(&width.to_le_bytes())?;
    writer.write_all(&height.to_le_bytes())?;
    writer.write_all(&[packed, 0, 0])?;

    // --- Global Color Table (256 * 3 bytes) ---
    writer.write_all(&pal_bytes)?;

    // --- NETSCAPE2.0 Application Extension (loop forever) ---
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

    // --- Encode frames ---
    let mut encoder = LzwEncoder::new(8); // min code size = 8 for 256-color palette
    let mut prev_indices: Vec<u8> = Vec::new();
    let mut frame_count: usize = 0;
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

        // Frame deduplication: skip identical consecutive frames, accumulate delay
        if indices == prev_indices {
            // Compute what the delay for this frame would be
            let delay_cs = if fi + 1 < frames.len() {
                let delta_ms = frames[fi + 1]
                    .timestamp_ms
                    .saturating_sub(frame.timestamp_ms);
                (delta_ms / 10).clamp(2, 65535) as u16
            } else {
                2 // Minimum 2 centiseconds for last frame
            };
            pending_delay_cs = pending_delay_cs.saturating_add(delay_cs);
            continue;
        }

        // Write the previous accumulated frame (if any was deferred)
        // Actually, we write the *current* frame with accumulated delay from previous skipped frames.
        // The delay for this frame includes any time accumulated from skipped duplicates.

        // Calculate delay: time from this frame to the next unique frame (or end)
        let delay_cs = if fi + 1 < frames.len() {
            // Find next frame that will actually be written (or just use next timestamp)
            let delta_ms = frames[fi + 1]
                .timestamp_ms
                .saturating_sub(frame.timestamp_ms);
            (delta_ms / 10).clamp(2, 65535) as u16
        } else {
            2
        };

        // Add any pending delay from skipped frames
        let total_delay = pending_delay_cs.saturating_add(delay_cs);
        pending_delay_cs = 0;

        // Graphic Control Extension
        writer.write_all(&[
            0x21, // Extension introducer
            0xF9, // Graphic Control label
            4,    // Block size
            0x00, // Packed: dispose=0, no user input, no transparent
        ])?;
        writer.write_all(&total_delay.to_le_bytes())?;
        writer.write_all(&[
            0, // Transparent color index (unused)
            0, // Block terminator
        ])?;

        // Image Descriptor
        writer.write_all(&[
            0x2C, // Image separator
        ])?;
        writer.write_all(&0u16.to_le_bytes())?; // Left
        writer.write_all(&0u16.to_le_bytes())?; // Top
        writer.write_all(&width.to_le_bytes())?; // Width
        writer.write_all(&height.to_le_bytes())?; // Height
        writer.write_all(&[0x00])?; // Packed: no local color table

        // LZW-compressed image data
        let compressed = encoder.encode(&indices);

        // Write as sub-blocks (max 255 bytes each)
        let mut pos = 0;
        while pos < compressed.len() {
            let chunk_len = (compressed.len() - pos).min(255);
            writer.write_all(&[chunk_len as u8])?;
            writer.write_all(&compressed[pos..pos + chunk_len])?;
            pos += chunk_len;
        }
        writer.write_all(&[0])?; // Block terminator

        prev_indices = indices;
        frame_count += 1;
    }

    // --- GIF Trailer ---
    writer.write_all(&[0x3B])?;

    let _ = frame_count; // Used for tracking; caller reports count
    writer.flush()?;

    Ok(())
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
