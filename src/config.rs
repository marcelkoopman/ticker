use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub price_path: String,
    pub unit: String,
    pub unit_hint: String,
    pub symbol: String,
}

pub fn bundled_config_path() -> Result<PathBuf, Box<dyn Error>> {
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

/// Kept for older call sites / tests.
pub fn config_path() -> Result<PathBuf, Box<dyn Error>> {
    if user_config_path()?.exists() {
        user_config_path()
    } else {
        bundled_config_path()
    }
}

pub fn user_config_path() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(path) = std::env::var("TICKER_USER_CONFIG_PATH")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".ticker_config.toml"))
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
    let user = user_config_path()?;
    let path = if user.exists() {
        eprintln!("📂 Reading user config from: {:?}", user);
        user
    } else {
        let bundled = bundled_config_path()?;
        eprintln!("📂 Reading bundled config from: {:?}", bundled);
        bundled
    };
    let mut config = load_config_from(&path)?;
    if let Some(pin) = load_menubar_pin() {
        config.menubar_asset = Some(pin);
    }
    Ok(config)
}

pub fn save_user_config(config: &Config) -> Result<PathBuf, Box<dyn Error>> {
    let path = user_config_path()?;
    let body = toml::to_string_pretty(config)?;
    fs::write(&path, body)?;
    Ok(path)
}

pub fn reset_user_config() -> Result<Config, Box<dyn Error>> {
    let path = user_config_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    load_config()
}

/// Switch a fetchable asset to EUR / USD / GBP by rewriting its URL and unit.
/// Fuel and power feeds are source-locked and return an error.
pub fn apply_quote_currency(asset: &mut Asset, code: &str) -> Result<(), String> {
    let code = code.trim().to_uppercase();
    let lower = code.to_lowercase();
    if !matches!(code.as_str(), "EUR" | "USD" | "GBP") {
        return Err(format!("Unsupported currency {code}"));
    }

    if asset.url.contains("coingecko.com") {
        asset.url = replace_query_value(&asset.url, "vs_currencies", &lower);
        if let Some((head, _)) = asset.price_path.rsplit_once('.') {
            asset.price_path = format!("{head}.{lower}");
        }
        asset.unit = code;
        return Ok(());
    }

    if asset.url.contains("xaus.com") {
        asset.url = replace_query_value(&asset.url, "currency", &code);
        asset.unit = code;
        return Ok(());
    }

    Err(format!(
        "{} is only available in {}",
        asset.name, asset.unit
    ))
}

fn replace_query_value(url: &str, key: &str, value: &str) -> String {
    let needle = format!("{key}=");
    if let Some(start) = url.find(&needle) {
        let val_start = start + needle.len();
        let val_end = url[val_start..]
            .find('&')
            .map(|i| val_start + i)
            .unwrap_or(url.len());
        format!("{}{}{}", &url[..val_start], value, &url[val_end..])
    } else if url.contains('?') {
        format!("{url}&{key}={value}")
    } else {
        format!("{url}?{key}={value}")
    }
}

fn menubar_pin_path() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(path) = std::env::var("TICKER_MENUBAR_PIN_PATH")
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    Ok(home.join(".ticker_menubar_asset"))
}

/// Last asset the user pinned by clicking a price row.
pub fn load_menubar_pin() -> Option<String> {
    let path = menubar_pin_path().ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let name = raw.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub fn save_menubar_pin(name: &str) -> Result<(), Box<dyn Error>> {
    let path = menubar_pin_path()?;
    fs::write(path, name.trim())?;
    Ok(())
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

    fn gecko_btc() -> Asset {
        Asset {
            name: "Bitcoin".into(),
            url: "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=eur"
                .into(),
            price_path: "bitcoin.eur".into(),
            unit: "EUR".into(),
            unit_hint: "/BTC".into(),
            symbol: "💰".into(),
        }
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

    #[test]
    fn menubar_pin_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ticker-menubar-pin-{stamp}"));
        unsafe {
            std::env::set_var("TICKER_MENUBAR_PIN_PATH", &path);
        }
        save_menubar_pin("Gold").expect("save");
        assert_eq!(load_menubar_pin().as_deref(), Some("Gold"));
        let _ = fs::remove_file(&path);
        unsafe {
            std::env::remove_var("TICKER_MENUBAR_PIN_PATH");
        }
    }

    #[test]
    fn apply_quote_currency_coingecko_usd() {
        let mut asset = gecko_btc();
        apply_quote_currency(&mut asset, "usd").unwrap();
        assert!(asset.url.contains("vs_currencies=usd"));
        assert!(!asset.url.contains("vs_currencies=eur"));
        assert_eq!(asset.price_path, "bitcoin.usd");
        assert_eq!(asset.unit, "USD");
    }

    #[test]
    fn apply_quote_currency_xaus() {
        let mut asset = Asset {
            name: "Gold".into(),
            url: "https://xaus.com/api/v1/spot?currency=EUR".into(),
            price_path: "xau.price".into(),
            unit: "EUR".into(),
            unit_hint: "/troy oz".into(),
            symbol: "🥇".into(),
        };
        apply_quote_currency(&mut asset, "GBP").unwrap();
        assert!(asset.url.contains("currency=GBP"));
        assert_eq!(asset.unit, "GBP");
    }

    #[test]
    fn apply_quote_currency_rejects_locked_feed() {
        let mut asset = Asset {
            name: "Benzine".into(),
            url: "https://eurooilwatch.com/api/v1/prices".into(),
            price_path: "countries.countryCode=NL.petrolPrice".into(),
            unit: "EUR".into(),
            unit_hint: "/L".into(),
            symbol: "⛽".into(),
        };
        assert!(apply_quote_currency(&mut asset, "USD").is_err());
        assert_eq!(asset.unit, "EUR");
    }

    #[test]
    fn user_config_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ticker-user-config-{stamp}.toml"));
        unsafe {
            std::env::set_var("TICKER_USER_CONFIG_PATH", &path);
        }
        let mut config = parse_config(sample_toml()).unwrap();
        apply_quote_currency(&mut config.assets[0], "USD").unwrap();
        save_user_config(&config).unwrap();
        let loaded = load_config_from(&path).unwrap();
        assert_eq!(loaded.assets[0].unit, "USD");
        reset_user_config().unwrap();
        assert!(!path.exists());
        unsafe {
            std::env::remove_var("TICKER_USER_CONFIG_PATH");
        }
    }
}
