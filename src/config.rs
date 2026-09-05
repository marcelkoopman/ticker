use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub price_path: String,
    pub unit: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub assets: Vec<Asset>,
}

fn default_poll_interval() -> String {
    "1h".to_string()
}

pub fn load_config() -> Result<Config, Box<dyn Error>> {
    let config_path = config_path()?;
    eprintln!("📂 Reading config from: {:?}", config_path);
    let config_str = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {:?}: {}", config_path, e))?;
    Ok(toml::from_str(&config_str)?)
}

fn config_path() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = std::env::current_exe()?;
    if let Some(app_dir) = exe_path.ancestors().find(|p| {
        p.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".app"))
            .unwrap_or(false)
    }) {
        return Ok(app_dir.join("Contents/Resources/config.toml"));
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml"))
}
