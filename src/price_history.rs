use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

pub fn load_price_history() -> Result<HashMap<String, PriceSnapshot>, Box<dyn Error>> {
    let path = price_history_path();

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let data = fs::read_to_string(&path)?;
    let history: PriceHistory = serde_json::from_str(&data)?;
    Ok(history.prices)
}

pub fn save_price_history(prices: &HashMap<String, PriceSnapshot>) -> Result<(), Box<dyn Error>> {
    let path = price_history_path();
    let history = PriceHistory {
        prices: prices.clone(),
    };
    fs::write(&path, serde_json::to_string_pretty(&history)?)?;
    Ok(())
}

pub fn update_price_history(name: &str, price: f64) -> Result<(), Box<dyn Error>> {
    let mut history = load_price_history()?;
    history.insert(name.to_string(), PriceSnapshot { value: price });
    save_price_history(&history)?;
    Ok(())
}
