//! Hand-written PNG encoder with STORE-only zlib compression.
//! No external crate dependencies. Writes RGBA images.

use std::io::Write;

/// Write an RGBA image as a PNG file.
///
/// `pixels` is row-major RGBA data (4 bytes per pixel).
/// `width` and `height` are in pixels.
pub fn export_png<W: Write>(
    writer: &mut W,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> std::io::Result<()> {
    assert_eq!(
        pixels.len(),
        width as usize * height as usize * 4,
        "pixel data size mismatch"
    );

    // PNG signature
    writer.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;

    // IHDR chunk
    let mut ihdr_data = [0u8; 13];
    ihdr_data[0..4].copy_from_slice(&width.to_be_bytes());
    ihdr_data[4..8].copy_from_slice(&height.to_be_bytes());
    ihdr_data[8] = 8; // bit depth
    ihdr_data[9] = 6; // color type: RGBA
    ihdr_data[10] = 0; // compression: deflate
    ihdr_data[11] = 0; // filter: adaptive
    ihdr_data[12] = 0; // interlace: none
    write_chunk(writer, b"IHDR", &ihdr_data)?;

    // IDAT chunk — zlib-compressed filtered scanlines
    let raw_size = (width as usize * 4 + 1) * height as usize;
    let mut filtered = Vec::with_capacity(raw_size);
    for row in 0..height as usize {
        filtered.push(0); // filter type: None
        let start = row * width as usize * 4;
        let end = start + width as usize * 4;
        filtered.extend_from_slice(&pixels[start..end]);
    }

    let compressed = zlib_store_compress(&filtered);
    write_chunk(writer, b"IDAT", &compressed)?;

    // IEND chunk
    write_chunk(writer, b"IEND", &[])?;

    writer.flush()?;
    Ok(())
}

fn write_chunk<W: Write>(writer: &mut W, chunk_type: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
    writer.write_all(&(data.len() as u32).to_be_bytes())?;
    writer.write_all(chunk_type)?;
    writer.write_all(data)?;

    // PNG CRC32 covers chunk_type + data, in a single pass with one finalize.
    let mut crc = 0xFFFFFFFFu32;
    for &byte in chunk_type.iter().chain(data.iter()) {
        crc = CRC_TABLE[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    let crc = !crc;
    writer.write_all(&crc.to_be_bytes())?;
    Ok(())
}

/// Minimal CRC32 (ISO 3309 / ITU-T V.42) for PNG chunk checksums.
/// Retained for tests; production CRC is computed inline in `write_chunk`.
#[cfg(test)]
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc = CRC_TABLE[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

const CRC_TABLE: [u32; 256] = generate_crc_table();

const fn generate_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut j = 0;
        while j < 8 {
            if c & 1 != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            j += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// Zlib STORE-only compression (no DEFLATE).
/// Format: CMF byte + FLG byte + stored blocks + Adler-32 checksum.
fn zlib_store_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 11);
    // zlib header: CMF=0x78 (deflate, window=32K), FLG=0x01 (no dict, check bits valid)
    out.push(0x78);
    out.push(0x01);

    // Write stored blocks (max 65535 bytes each)
    let mut pos = 0;
    while pos < data.len() {
        let remaining = data.len() - pos;
        let block_len = remaining.min(65535);
        let is_last = pos + block_len == data.len();

        out.push(if is_last { 0x01 } else { 0x00 }); // BFINAL + BTYPE=00 (stored)
        out.extend_from_slice(&(block_len as u16).to_le_bytes());
        out.extend_from_slice(&(!(block_len as u16)).to_le_bytes());
        out.extend_from_slice(&data[pos..pos + block_len]);

        pos += block_len;
    }

    // Adler-32 checksum
    let adler = adler32(data);
    out.extend_from_slice(&adler.to_be_bytes());

    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_header_valid() {
        let mut buf = Vec::new();
        let pixels = vec![0u8; 4 * 4]; // 2x2 RGBA
        export_png(&mut buf, &pixels, 2, 2).unwrap();
        // PNG signature
        assert_eq!(&buf[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        // IHDR chunk type
        assert_eq!(&buf[12..16], b"IHDR");
        // IEND at end
        assert!(buf.len() > 8);
        let len = buf.len();
        assert_eq!(&buf[len - 8..len - 4], b"IEND");
    }

    #[test]
    fn test_png_pixel_data_roundtrip() {
        let pixels: Vec<u8> = vec![
            255, 0, 0, 255, // red
            0, 0, 255, 255, // blue
        ];
        let mut buf = Vec::new();
        export_png(&mut buf, &pixels, 2, 1).unwrap();
        assert!(buf.len() > 40);
    }

    #[test]
    fn test_adler32_known() {
        assert_eq!(adler32(&[]), 1);
        assert_eq!(adler32(b"abc"), 0x024D0127);
    }

    #[test]
    fn test_crc32_known() {
        assert_eq!(crc32(&[]), 0x00000000);
        assert_eq!(crc32(b"IEND"), 0xAE426082);
    }

    #[test]
    fn test_chunk_crcs_validate() {
        // Encode a 3x2 RGBA image and verify every chunk's stored CRC matches
        // a fresh CRC32 over (chunk_type ++ data) — strict PNG decoders enforce
        // this and reject the file otherwise.
        let pixels: Vec<u8> = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, // row 0
            255, 255, 0, 255, 0, 255, 255, 255, 255, 0, 255, 255, // row 1
        ];
        let mut buf = Vec::new();
        export_png(&mut buf, &pixels, 3, 2).unwrap();

        let mut pos = 8; // skip PNG signature
        let mut chunks_checked = 0;
        while pos + 8 <= buf.len() {
            let len = u32::from_be_bytes(buf[pos..pos + 4].try_into().unwrap()) as usize;
            let chunk_start = pos + 4;
            let data_start = chunk_start + 4;
            let crc_start = data_start + len;
            let crc_end = crc_start + 4;
            assert!(crc_end <= buf.len(), "chunk extends past file end");

            // Recompute CRC over chunk_type + data
            let mut crc = 0xFFFFFFFFu32;
            for &b in &buf[chunk_start..crc_start] {
                crc = CRC_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
            }
            let expected = !crc;
            let stored = u32::from_be_bytes(buf[crc_start..crc_end].try_into().unwrap());
            let chunk_type = std::str::from_utf8(&buf[chunk_start..data_start]).unwrap();
            assert_eq!(
                stored, expected,
                "CRC mismatch for chunk {}: stored {:08x}, expected {:08x}",
                chunk_type, stored, expected
            );

            chunks_checked += 1;
            pos = crc_end;
            if chunk_type == "IEND" {
                break;
            }
        }
        // IHDR + IDAT + IEND minimum
        assert!(
            chunks_checked >= 3,
            "expected at least 3 chunks, got {}",
            chunks_checked
        );
    }

    #[test]
    fn test_size_mismatch_panics() {
        let result = std::panic::catch_unwind(|| {
            let mut buf = Vec::new();
            let pixels = vec![0u8; 8]; // 2 pixels, but claiming 3x1
            export_png(&mut buf, &pixels, 3, 1).unwrap();
        });
        assert!(result.is_err());
    }
}
