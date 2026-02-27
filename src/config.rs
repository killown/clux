use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
pub struct CluxConfig {
    pub keyboard: KeyboardConfig,
    pub keybindings: HashMap<String, Keybinding>,
    #[serde(default)]
    pub outputs: Vec<OutputConfig>,
}

#[derive(Deserialize)]
pub struct OutputConfig {
    pub name: String,
    pub pos: (i32, i32),
    #[serde(default = "default_scale")]
    pub scale: f32,
}

fn default_scale() -> f32 {
    1.0
}

#[derive(Deserialize)]
pub struct KeyboardConfig {
    pub layout: String,
    pub variant: String,
    pub options: Option<String>,
}

#[derive(Deserialize)]
pub struct Keybinding {
    pub combo: String,
    pub command: String,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            layout: "us".to_string(),
            variant: "".to_string(),
            options: None,
        }
    }
}

pub fn load_config() -> CluxConfig {
    let config_dir: PathBuf = dirs::config_dir()
        .map(|p| p.join("clux"))
        .unwrap_or_else(|| "/etc/clux".into());

    let config_path = config_dir.join("config.toml");

    if let Ok(content) = fs::read_to_string(config_path) {
        match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                tracing::error!("Config parsing error: {}", e);
                CluxConfig::default()
            }
        }
    } else {
        CluxConfig::default()
    }
}
