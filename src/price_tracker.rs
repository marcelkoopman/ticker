use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinnedPrice {
    pub value: f64,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinnedPrices {
    prices: HashMap<String, PinnedPrice>,
}

fn pinned_prices_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".ticker_pinned_prices.json")
}

pub fn load_pinned_prices() -> Result<HashMap<String, PinnedPrice>, Box<dyn Error>> {
    let path = pinned_prices_path();

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let data = fs::read_to_string(&path)?;
    let pinned: PinnedPrices = serde_json::from_str(&data)?;
    Ok(pinned.prices)
}

pub fn save_pinned_price(name: &str, price: f64) -> Result<(), Box<dyn Error>> {
    let mut pinned = load_pinned_prices()?;

    pinned.insert(
        name.to_string(),
        PinnedPrice {
            value: price,
            timestamp: Local::now().format("%H:%M:%S").to_string(),
        },
    );

    let path = pinned_prices_path();
    let data = PinnedPrices { prices: pinned };
    fs::write(&path, serde_json::to_string_pretty(&data)?)?;

    Ok(())
}

pub fn remove_pinned_price(name: &str) -> Result<(), Box<dyn Error>> {
    let mut pinned = load_pinned_prices()?;
    pinned.remove(name);

    let path = pinned_prices_path();
    let data = PinnedPrices { prices: pinned };
    fs::write(&path, serde_json::to_string_pretty(&data)?)?;

    Ok(())
}

pub fn get_price_change(name: &str, current_price: f64) -> Option<String> {
    let pinned = load_pinned_prices().ok()?;

    if let Some(pinned_price) = pinned.get(name) {
        let difference = current_price - pinned_price.value;

        if difference > 0.0 {
            return Some(format!("🟢 +{:.2}", difference)); // Groen voor stijging
        } else if difference < 0.0 {
            return Some(format!("🔴 {:.2}", difference)); // Rood voor daling
        } else {
            return Some("⚪".to_string()); // Grijs/wit voor gelijk
        }
    }

    None
}
