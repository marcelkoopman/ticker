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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_history() {
        let result = load_price_history();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_save_and_load_price() {
        let mut prices = HashMap::new();
        prices.insert("Bitcoin".to_string(), PriceSnapshot { value: 50_000.0 });

        let result = save_price_history(&prices);
        assert!(result.is_ok());

        let loaded = load_price_history();
        assert!(loaded.is_ok());

        let loaded_prices = loaded.unwrap();
        assert!(loaded_prices.contains_key("Bitcoin"));
        assert_eq!(loaded_prices["Bitcoin"].value, 50_000.0);

        let _ = clear_price_history();
    }

    #[test]
    fn test_update_price_history() {
        let _ = clear_price_history();

        let result = update_price_history("Gold", 3_800.0);
        assert!(result.is_ok());

        let loaded = load_price_history().unwrap();
        assert!(loaded.contains_key("Gold"));
        assert_eq!(loaded["Gold"].value, 3_800.0);

        let _ = clear_price_history();
    }

    #[test]
    fn test_delta_positive() {
        let _ = clear_price_history();

        let _ = update_price_history("Bitcoin", 50_000.0);

        let delta = get_delta("Bitcoin", 51_000.0);
        assert!(delta.is_ok());

        let delta_str = delta.unwrap();
        assert!(delta_str.contains("🟢"));
        assert!(delta_str.contains("+1000.00"));

        let _ = clear_price_history();
    }

    #[test]
    fn test_delta_negative() {
        let _ = clear_price_history();

        let _ = update_price_history("Gold", 3_800.0);

        let delta = get_delta("Gold", 3_700.0);
        assert!(delta.is_ok());

        let delta_str = delta.unwrap();
        assert!(delta_str.contains("🔴"));
        assert!(delta_str.contains("-100.00"));

        let _ = clear_price_history();
    }

    #[test]
    fn test_delta_no_change() {
        let _ = clear_price_history();

        let _ = update_price_history("TTF Gas", 72.5);

        let delta = get_delta("TTF Gas", 72.5);
        assert!(delta.is_ok());

        let delta_str = delta.unwrap();
        assert_eq!(delta_str, "⚪");

        let _ = clear_price_history();
    }

    #[test]
    fn test_delta_no_previous_price() {
        let _ = clear_price_history();

        let delta = get_delta("Unknown", 100.0);
        assert!(delta.is_ok());
        assert_eq!(delta.unwrap(), "⚪");

        let _ = clear_price_history();
    }

    #[test]
    fn test_delta_percentage_calculation() {
        let _ = clear_price_history();

        let _ = update_price_history("Bitcoin", 50_000.0);

        let delta = get_delta("Bitcoin", 55_000.0);
        assert!(delta.is_ok());

        let delta_str = delta.unwrap();
        assert!(delta_str.contains("10.00%"));

        let _ = clear_price_history();
    }

    #[test]
    fn test_delta_simple() {
        let _ = clear_price_history();

        let _ = update_price_history("Gold", 3_800.0);

        let delta = get_delta_simple("Gold", 3_850.0);
        assert!(delta.is_ok());

        let delta_str = delta.unwrap();
        assert!(delta_str.contains("🟢"));
        assert!(delta_str.contains("+50.00"));
        assert!(!delta_str.contains("%"));

        let _ = clear_price_history();
    }

    #[test]
    fn test_clear_price_history() {
        let _ = clear_price_history();

        let _ = update_price_history("Bitcoin", 50_000.0);
        let loaded = load_price_history().unwrap();
        assert!(!loaded.is_empty());

        let result = clear_price_history();
        assert!(result.is_ok());

        let loaded = load_price_history().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_multiple_assets() {
        let _ = clear_price_history();

        let _ = update_price_history("Bitcoin", 50_000.0);
        let _ = update_price_history("Gold", 3_800.0);
        let _ = update_price_history("TTF Gas", 72.5);

        let loaded = load_price_history().unwrap();
        assert_eq!(loaded.len(), 3);
        assert!(loaded.contains_key("Bitcoin"));
        assert!(loaded.contains_key("Gold"));
        assert!(loaded.contains_key("TTF Gas"));

        let _ = clear_price_history();
    }

    #[test]
    fn test_floating_point_tolerance() {
        let _ = clear_price_history();

        let _ = update_price_history("Bitcoin", 50_000.0);

        let delta = get_delta("Bitcoin", 50_000.0005);
        assert!(delta.is_ok());
        assert_eq!(delta.unwrap(), "⚪");

        let _ = clear_price_history();
    }
}
