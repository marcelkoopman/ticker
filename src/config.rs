use crate::models::Config;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

pub fn config_path() -> Result<PathBuf, Box<dyn Error>> {
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

pub fn load_config() -> Result<Config, Box<dyn Error>> {
    eprintln!("📋 Looking for config.toml...");
    let path = config_path()?;
    eprintln!("📂 Reading config from: {:?}", path);

    let config_str =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

    toml::from_str(&config_str).map_err(|e| e.into())
}
