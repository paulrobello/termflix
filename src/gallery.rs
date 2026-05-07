use crate::animations;
use crate::gif;
use crate::png;
use crate::render::{Canvas, ColorMode, PostProcessConfig, RenderMode};
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

const PNG_SCALE: usize = 8;
const GIF_SCALE: usize = 8;

pub struct GalleryConfig {
    pub dir: PathBuf,
    pub cols: usize,
    pub rows: usize,
    pub wait_secs: f64,
    pub duration_secs: f64,
    pub names: Option<Vec<String>>,
}

pub fn run_gallery(config: &GalleryConfig) -> std::io::Result<()> {
    fs::create_dir_all(&config.dir)?;

    let anim_list = resolve_animations(config);
    if anim_list.is_empty() {
        eprintln!("No animations to capture.");
        return Ok(());
    }

    eprintln!(
        "Capturing {} animation(s) to {}...",
        anim_list.len(),
        config.dir.display()
    );

    let mut captured: Vec<(String, String)> = Vec::new();

    for (i, (name, desc)) in anim_list.iter().enumerate() {
        eprintln!("[{}/{}] {}...", i + 1, anim_list.len(), name);
        match capture_animation(name, config) {
            Ok(()) => captured.push((name.to_string(), desc.to_string())),
            Err(e) => eprintln!("  ERROR: {} — skipping ({})", name, e),
        }
    }

    generate_index_html(&captured, config)?;
    eprintln!(
        "\nDone! {} animation(s) captured. Gallery at {}/index.html",
        captured.len(),
        config.dir.display()
    );
    Ok(())
}

fn resolve_animations(config: &GalleryConfig) -> Vec<(&'static str, &'static str)> {
    if let Some(ref names) = config.names {
        let mut result: Vec<(&'static str, &'static str)> = Vec::new();
        for requested in names {
            if let Some(entry) = animations::ANIMATIONS
                .iter()
                .find(|(name, _)| *name == requested.as_str())
            {
                result.push((entry.0, entry.1));
            } else {
                eprintln!("Warning: unknown animation '{}', skipping", requested);
            }
        }
        result
    } else {
        animations::ANIMATIONS.to_vec()
    }
}

fn capture_animation(name: &str, config: &GalleryConfig) -> std::io::Result<()> {
    let cols = config.cols;
    let rows = config.rows;
    let fps = 24.0;
    let dt = 1.0 / fps;
    let total_frames = (config.duration_secs * fps) as usize;
    let png_frame = (config.wait_secs * fps) as usize;
    let png_frame = png_frame.clamp(0, total_frames.saturating_sub(1));

    let color_mode = ColorMode::TrueColor;
    let render_mode = RenderMode::HalfBlock;
    let canvas = Canvas::new(cols, rows, render_mode, color_mode);

    let mut anim = animations::create(name, canvas.width, canvas.height, 1.0).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown animation: {}", name),
        )
    })?;
    let mut canvas = Canvas::new(cols, rows, render_mode, color_mode);
    anim.on_resize(canvas.width, canvas.height);

    let postproc = PostProcessConfig {
        bloom: 0.4,
        bloom_threshold: 0.6,
        vignette: 0.0,
        scanlines: false,
    };

    let mut gif_frames: Vec<gif::PixelFrame> = Vec::with_capacity(total_frames);
    let mut png_frame_pixels: Option<Vec<(u8, u8, u8)>> = None;
    let mut time = 0.0f64;
    let dt_ms = (dt * 1000.0) as u64;

    for frame_i in 0..total_frames {
        canvas.clear();
        anim.update(&mut canvas, dt, time);
        canvas.apply_effects(1.0, 0.0);
        canvas.post_process(&postproc);

        // Convert canvas (brightness * RGB) to flat RGB for this frame.
        let mut frame_pixels: Vec<(u8, u8, u8)> = Vec::with_capacity(canvas.width * canvas.height);
        for idx in 0..canvas.pixels.len() {
            let v = canvas.pixels[idx].clamp(0.0, 1.0);
            let (r, g, b) = canvas.colors[idx];
            frame_pixels.push((
                (r as f64 * v) as u8,
                (g as f64 * v) as u8,
                (b as f64 * v) as u8,
            ));
        }

        if frame_i == png_frame {
            png_frame_pixels = Some(frame_pixels.clone());
        }

        gif_frames.push(gif::PixelFrame {
            timestamp_ms: frame_i as u64 * dt_ms,
            pixels: frame_pixels,
        });

        time += dt;
    }

    // Write PNG — render the chosen still frame at PNG_SCALE.
    if let Some(snap) = png_frame_pixels {
        let img_w = canvas.width * PNG_SCALE;
        let img_h = canvas.height * PNG_SCALE;
        let stride = img_w * 4;
        let mut pixels = vec![0u8; img_w * img_h * 4];

        for cy in 0..canvas.height {
            for cx in 0..canvas.width {
                let (r, g, b) = snap[cy * canvas.width + cx];
                let base_y = cy * PNG_SCALE;
                let base_x = cx * PNG_SCALE;
                for dy in 0..PNG_SCALE {
                    let row_start = (base_y + dy) * stride + base_x * 4;
                    for dx in 0..PNG_SCALE {
                        let pidx = row_start + dx * 4;
                        pixels[pidx] = r;
                        pixels[pidx + 1] = g;
                        pixels[pidx + 2] = b;
                        pixels[pidx + 3] = 255;
                    }
                }
            }
        }

        let png_path = config.dir.join(format!("{}.png", name));
        let file = fs::File::create(&png_path)?;
        let mut writer = BufWriter::new(file);
        png::export_png(&mut writer, &pixels, img_w as u32, img_h as u32)?;
    }

    // Write GIF — pixel-based path; bypasses VirtualTerminal so BG/half-block
    // colors aren't lost and the output is at canvas resolution × GIF_SCALE.
    let gif_path = config.dir.join(format!("{}.gif", name));
    let file = fs::File::create(&gif_path)?;
    let mut writer = BufWriter::new(file);
    gif::export_gif_pixels(
        &mut writer,
        &gif_frames,
        canvas.width,
        canvas.height,
        GIF_SCALE,
    )?;

    Ok(())
}

fn generate_index_html(
    animations: &[(String, String)],
    config: &GalleryConfig,
) -> std::io::Result<()> {
    let mut html = String::with_capacity(animations.len() * 500);

    html.push_str(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>termflix gallery</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { background: #0d1117; color: #e6edf3; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 24px; }
  h1 { text-align: center; font-size: 28px; margin-bottom: 4px; color: #58a6ff; }
  .subtitle { text-align: center; color: #8b949e; margin-bottom: 24px; font-size: 14px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 16px; max-width: 1200px; margin: 0 auto; }
  .card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; overflow: hidden; cursor: pointer; transition: border-color 0.2s; }
  .card:hover { border-color: #58a6ff; }
  .card img { width: 100%; display: block; background: #010409; }
  .card-body { padding: 8px 12px; }
  .card-name { font-weight: 600; font-size: 14px; }
  .card-desc { color: #8b949e; font-size: 11px; margin-top: 2px; }
  .footer { text-align: center; margin-top: 24px; color: #8b949e; font-size: 12px; }
  code { background: #161b22; padding: 2px 6px; border-radius: 3px; }
  .lightbox { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.9); z-index: 100; cursor: pointer; }
  .lightbox.active { display: flex; align-items: center; justify-content: center; }
  .lightbox img { max-width: 90%; max-height: 90%; border-radius: 8px; }
  .lightbox .close { position: absolute; top: 16px; right: 24px; color: #8b949e; font-size: 24px; cursor: pointer; }
</style>
</head>
<body>
<h1>termflix gallery</h1>
<p class="subtitle">"#,
    );
    html.push_str(&format!(
        "{} terminal animations — click any card to see it animated",
        animations.len()
    ));
    html.push_str(
        r#"</p>
<div class="grid">
"#,
    );

    for (name, desc) in animations {
        html.push_str(r#"  <div class="card" onclick="openLightbox('"#);
        html.push_str(name);
        html.push_str(
            r#"')">
    <img src=""#,
        );
        html.push_str(name);
        html.push_str(r#".png" alt=""#);
        html.push_str(name);
        html.push_str(
            r#"" loading="lazy">
    <div class="card-body">
      <div class="card-name">"#,
        );
        html.push_str(name);
        html.push_str(
            r#"</div>
      <div class="card-desc">"#,
        );
        html.push_str(desc);
        html.push_str(
            r#"</div>
    </div>
  </div>
"#,
        );
    }

    html.push_str(
        r#"</div>
<div class="footer">Generated by <code>termflix --gallery</code></div>
<div class="lightbox" id="lightbox" onclick="closeLightbox()">
  <span class="close">&times;</span>
  <img id="lightbox-img" src="" alt="animated">
</div>
<script>
function openLightbox(name) {
  var lb = document.getElementById('lightbox');
  var img = document.getElementById('lightbox-img');
  img.src = name + '.gif';
  lb.classList.add('active');
}
function closeLightbox() {
  var lb = document.getElementById('lightbox');
  lb.classList.remove('active');
  document.getElementById('lightbox-img').src = '';
}
document.addEventListener('keydown', function(e) { if (e.key === 'Escape') closeLightbox(); });
</script>
</body>
</html>"#,
    );

    let path = config.dir.join("index.html");
    fs::write(path, html)?;
    Ok(())
}
