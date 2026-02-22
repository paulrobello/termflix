use crate::render::{ColorMode, RenderMode};
use serde::Deserialize;
use std::path::PathBuf;

/// User configuration loaded from config file.
/// All fields are optional — CLI flags override config, config overrides defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Default animation name
    pub animation: Option<String>,
    /// Default render mode
    pub render: Option<RenderModeConfig>,
    /// Default color mode
    pub color: Option<ColorModeConfig>,
    /// Target FPS (1-120)
    pub fps: Option<u32>,
    /// Particle/element scale factor (0.5-2.0)
    pub scale: Option<f64>,
    /// Hide status bar
    pub clean: Option<bool>,
    /// Auto-cycle interval in seconds (0 = disabled)
    pub cycle: Option<u32>,
    /// Color quantization step (0 = off, 4/8/16 = coarser colors for less output)
    pub color_quant: Option<u8>,
}

/// Render mode names for config file (kebab-case friendly)
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderModeConfig {
    Braille,
    HalfBlock,
    Ascii,
}

impl From<RenderModeConfig> for RenderMode {
    fn from(c: RenderModeConfig) -> Self {
        match c {
            RenderModeConfig::Braille => RenderMode::Braille,
            RenderModeConfig::HalfBlock => RenderMode::HalfBlock,
            RenderModeConfig::Ascii => RenderMode::Ascii,
        }
    }
}

/// Color mode names for config file (kebab-case friendly)
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorModeConfig {
    Mono,
    Ansi16,
    Ansi256,
    TrueColor,
}

impl From<ColorModeConfig> for ColorMode {
    fn from(c: ColorModeConfig) -> Self {
        match c {
            ColorModeConfig::Mono => ColorMode::Mono,
            ColorModeConfig::Ansi16 => ColorMode::Ansi16,
            ColorModeConfig::Ansi256 => ColorMode::Ansi256,
            ColorModeConfig::TrueColor => ColorMode::TrueColor,
        }
    }
}

/// Get the config file path: ~/.config/termflix/config.toml
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("termflix").join("config.toml"))
}

/// Load config from file. Returns default config if file doesn't exist.
pub fn load_config() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    match toml::from_str(&contents) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: failed to parse {}: {}", path.display(), e);
            Config::default()
        }
    }
}

/// Generate a default config file with all options commented out
pub fn default_config_string() -> String {
    r#"# termflix configuration
# Use --show-config to see the active config file path.
# CLI flags override these settings.

# Default animation (use --list to see all)
# animation = "fire"

# Default render mode: braille, half-block, ascii
# render = "half-block"

# Default color mode: mono, ansi16, ansi256, true-color
# color = "true-color"

# Target FPS (1-120)
# fps = 24

# Particle/element scale factor (0.5-2.0)
# scale = 1.0

# Hide status bar
# clean = false

# Auto-cycle interval in seconds (0 = disabled)
# cycle = 0

# Color quantization step (0 = off, 4/8/16 = coarser colors, less output)
# Useful for slow terminals or tmux
# color_quant = 0
"#
    .to_string()
}
