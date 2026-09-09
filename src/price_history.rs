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

/// Persist price values to the default history file.
pub fn save_price_history(prices: &HashMap<String, f64>) -> Result<(), Box<dyn Error>> {
    let snapshots: HashMap<String, PriceSnapshot> = prices
        .iter()
        .map(|(name, value)| (name.clone(), PriceSnapshot { value: *value }))
        .collect();
    save_price_history_to(&price_history_path(), &snapshots)
}
