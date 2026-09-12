use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Name of the asset shown in the menu-bar title. Defaults to first priced row.
    #[serde(default)]
    pub menubar_asset: Option<String>,
    pub assets: Vec<Asset>,
}

impl Config {
    pub fn menubar_asset_name(&self) -> Option<&str> {
        self.menubar_asset.as_deref().filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub price_path: String,
    pub unit: String,
    pub unit_hint: String,
    pub symbol: String,
}

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

pub fn parse_config(config_str: &str) -> Result<Config, Box<dyn Error>> {
    toml::from_str(config_str).map_err(|e| e.into())
}

pub fn load_config_from(path: &Path) -> Result<Config, Box<dyn Error>> {
    let config_str =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    parse_config(&config_str)
}

pub fn load_config() -> Result<Config, Box<dyn Error>> {
    eprintln!("📋 Looking for config.toml...");
    let path = config_path()?;
    eprintln!("📂 Reading config from: {:?}", path);
    load_config_from(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_toml() -> &'static str {
        r#"
[[assets]]
name = "Bitcoin"
url = "https://example.com/btc"
price_path = "bitcoin.eur"
unit = "EUR"
unit_hint = "/BTC"
symbol = "💰"

[[assets]]
name = "Gold"
url = "https://example.com/gold"
price_path = "xau.price"
unit = "EUR"
unit_hint = "/troy oz"
symbol = "🥇"
"#
    }

    #[test]
    fn parse_config_valid() {
        let config = parse_config(sample_toml()).expect("should parse");
        assert_eq!(config.assets.len(), 2);
        assert_eq!(config.assets[0].name, "Bitcoin");
        assert_eq!(config.assets[0].price_path, "bitcoin.eur");
        assert_eq!(config.assets[0].symbol, "💰");
        assert_eq!(config.assets[1].name, "Gold");
        assert_eq!(config.assets[1].unit, "EUR");
        assert!(config.menubar_asset.is_none());
    }

    #[test]
    fn parse_config_menubar_asset() {
        let toml = "menubar_asset = \"Gold\"\n\n[[assets]]\nname = \"Gold\"\nurl = \"https://example.com\"\nprice_path = \"xau.price\"\nunit = \"EUR\"\nunit_hint = \"/oz\"\nsymbol = \"🥇\"\n";
        let config = parse_config(toml).expect("should parse");
        assert_eq!(config.menubar_asset_name(), Some("Gold"));
    }

    #[test]
    fn parse_config_empty_assets() {
        let config = parse_config("assets = []").expect("should parse");
        assert!(config.assets.is_empty());
    }

    #[test]
    fn parse_config_invalid_toml() {
        assert!(parse_config("not valid toml {{{{").is_err());
    }

    #[test]
    fn parse_config_missing_required_field() {
        let bad = r#"
[[assets]]
name = "Bitcoin"
url = "https://example.com"
"#;
        assert!(parse_config(bad).is_err());
    }

    #[test]
    fn load_config_from_file() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ticker-config-test-{}.toml", stamp));
        fs::write(&path, sample_toml()).unwrap();

        let config = load_config_from(&path).expect("should load");
        assert_eq!(config.assets.len(), 2);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_config_from_missing_file() {
        let path = PathBuf::from("/tmp/ticker-config-does-not-exist-xyz.toml");
        assert!(load_config_from(&path).is_err());
    }

    #[test]
    fn config_path_returns_some_path() {
        let path = config_path().expect("should resolve");
        assert!(path.to_string_lossy().contains("config.toml"));
    }
}
