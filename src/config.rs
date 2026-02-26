use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
pub struct CluxConfig {
    pub keyboard: KeyboardConfig,
    pub keybindings: HashMap<String, Keybinding>,
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
        toml::from_str(&content).unwrap_or_default()
    } else {
        CluxConfig::default()
    }
}
