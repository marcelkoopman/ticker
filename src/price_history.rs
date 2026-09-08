use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PriceSnapshot {
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PriceHistory {
    prices: HashMap<String, PriceSnapshot>,
}

fn price_history_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".ticker_price_history.json")
}

/// Load price history from an explicit path.
pub fn load_price_history_from(
    path: &Path,
) -> Result<HashMap<String, PriceSnapshot>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let data = fs::read_to_string(path)?;
    let history: PriceHistory = serde_json::from_str(&data)?;
    Ok(history.prices)
}

/// Save price history to an explicit path.
pub fn save_price_history_to(
    path: &Path,
    prices: &HashMap<String, PriceSnapshot>,
) -> Result<(), Box<dyn Error>> {
    let history = PriceHistory {
        prices: prices.clone(),
    };
    let data = serde_json::to_string_pretty(&history)?;
    fs::write(path, data)?;
    Ok(())
}

pub fn load_price_history() -> Result<HashMap<String, PriceSnapshot>, Box<dyn Error>> {
    load_price_history_from(&price_history_path())
}

/// Persist price values to the default history file.
pub fn save_price_history(prices: &HashMap<String, f64>) -> Result<(), Box<dyn Error>> {
    let snapshots: HashMap<String, PriceSnapshot> = prices
        .iter()
        .map(|(name, value)| (name.clone(), PriceSnapshot { value: *value }))
        .collect();
    save_price_history_to(&price_history_path(), &snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_history_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ticker-history-{}-{}.json", name, stamp))
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = temp_history_path("missing");
        let _ = fs::remove_file(&path);
        let history = load_price_history_from(&path).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_history_path("roundtrip");
        let _ = fs::remove_file(&path);

        let mut prices = HashMap::new();
        prices.insert("Bitcoin".to_string(), PriceSnapshot { value: 95000.5 });
        prices.insert("Gold".to_string(), PriceSnapshot { value: 2650.0 });

        save_price_history_to(&path, &prices).unwrap();
        let loaded = load_price_history_from(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("Bitcoin").unwrap().value, 95000.5);
        assert_eq!(loaded.get("Gold").unwrap().value, 2650.0);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_invalid_json_errors() {
        let path = temp_history_path("invalid");
        fs::write(&path, "not-json{{{{").unwrap();
        assert!(load_price_history_from(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_empty_prices_object() {
        let path = temp_history_path("empty");
        fs::write(&path, r#"{"prices":{}}"#).unwrap();
        let history = load_price_history_from(&path).unwrap();
        assert!(history.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn overwrite_existing_history() {
        let path = temp_history_path("overwrite");
        let _ = fs::remove_file(&path);

        let mut first = HashMap::new();
        first.insert("BTC".to_string(), PriceSnapshot { value: 1.0 });
        save_price_history_to(&path, &first).unwrap();

        let mut second = HashMap::new();
        second.insert("ETH".to_string(), PriceSnapshot { value: 2.0 });
        save_price_history_to(&path, &second).unwrap();

        let loaded = load_price_history_from(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("ETH"));
        assert!(!loaded.contains_key("BTC"));

        let _ = fs::remove_file(&path);
    }
}
