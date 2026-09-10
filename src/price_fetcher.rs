use crate::config::Asset;
use polars::prelude::*;
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;
use std::thread;
use std::time::Duration;

const MAX_FETCH_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_millis(500);

pub struct PriceFetcher {
    client: Client,
}

impl PriceFetcher {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let client = Client::builder()
            .user_agent("rust-price-fetcher/1.0")
            .build()?;
        Ok(PriceFetcher { client })
    }

    pub fn fetch_price(&self, asset: &Asset) -> f64 {
        for attempt in 1..=MAX_FETCH_ATTEMPTS {
            let price = self.fetch_price_once(asset, attempt);
            if !price.is_nan() {
                return price;
            }

            if attempt < MAX_FETCH_ATTEMPTS {
                eprintln!(
                    "⚠️  {} failed (attempt {}/{}), retrying in {:?}...",
                    asset.name, attempt, MAX_FETCH_ATTEMPTS, RETRY_DELAY
                );
                thread::sleep(RETRY_DELAY);
            }
        }

        eprintln!(
            "✗ Giving up on {} after {} attempts",
            asset.name, MAX_FETCH_ATTEMPTS
        );
        f64::NAN
    }

    fn fetch_price_once(&self, asset: &Asset, attempt: u32) -> f64 {
        eprintln!(
            "🔍 Fetching {} from {} (attempt {}/{})",
            asset.name, asset.url, attempt, MAX_FETCH_ATTEMPTS
        );
        match self.client.get(&asset.url).send() {
            Ok(response) => match response.error_for_status() {
                Ok(resp) => match resp.json::<Value>() {
                    Ok(json) => {
                        let json_str = serde_json::to_string(&json).unwrap_or_default();
                        let preview_len = std::cmp::min(500, json_str.len());
                        eprintln!(
                            "📦 Raw JSON (first {} chars): {}",
                            preview_len,
                            &json_str[..preview_len]
                        );

                        if let Some(price_value) = self.get_value_by_path(&json, &asset.price_path)
                        {
                            match price_value {
                                Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
                                Value::String(s) => s.parse().unwrap_or(f64::NAN),
                                _ => f64::NAN,
                            }
                        } else {
                            eprintln!("✗ Path '{}' not found in JSON!", asset.price_path);
                            f64::NAN
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ JSON parse error: {}", e);
                        f64::NAN
                    }
                },
                Err(e) => {
                    eprintln!("✗ HTTP error: {}", e);
                    f64::NAN
                }
            },
            Err(e) => {
                eprintln!("✗ Network error: {}", e);
                f64::NAN
            }
        }
    }

    /// Fetch all assets into a base DataFrame: symbol, name, price, unit.
    pub fn fetch_all(&self, assets: &[Asset]) -> Result<DataFrame, Box<dyn Error>> {
        let mut symbols: Vec<String> = Vec::with_capacity(assets.len());
        let mut names: Vec<String> = Vec::with_capacity(assets.len());
        let mut prices: Vec<f64> = Vec::with_capacity(assets.len());
        let mut units: Vec<String> = Vec::with_capacity(assets.len());

        for asset in assets {
            let price = self.fetch_price(asset);
            symbols.push(asset.symbol.clone());
            names.push(asset.name.clone());
            prices.push(price);
            units.push(asset.unit.clone());
        }

        DataFrame::new(vec![
            Series::new("symbol".into(), symbols).into(),
            Series::new("name".into(), names).into(),
            Series::new("price".into(), prices).into(),
            Series::new("unit".into(), units).into(),
        ])
        .map_err(|e| e.into())
    }

    /// First poll: construct DataFrame with change columns (no previous → nulls / empty direction).
    pub fn build_initial_dataframe(&self, assets: &[Asset]) -> Result<DataFrame, Box<dyn Error>> {
        let base = self.fetch_all(assets)?;
        let n = base.height();
        let mut df = base;

        df.with_column(Series::new("prev_price".into(), vec![None::<f64>; n]).into())?;
        df.with_column(Series::new("change".into(), vec![None::<f64>; n]).into())?;
        df.with_column(Series::new("pct_change".into(), vec![None::<f64>; n]).into())?;
        df.with_column(Series::new("direction".into(), vec![String::new(); n]).into())?;

        eprintln!("📊 Initial DataFrame:\n{df}");
        Ok(df)
    }

    /// Subsequent poll: update prices and recompute change columns from the previous DataFrame.
    pub fn update_dataframe(
        &self,
        previous: &DataFrame,
        assets: &[Asset],
    ) -> Result<DataFrame, Box<dyn Error>> {
        let fresh = self.fetch_all(assets)?;

        // prev_price comes from previous.price, matched by name
        let prev_names = previous.column("name")?.str()?;
        let prev_prices = previous.column("price")?.f64()?;

        let mut prev_by_name: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for i in 0..previous.height() {
            if let (Some(name), Some(price)) = (prev_names.get(i), prev_prices.get(i)) {
                prev_by_name.insert(name.to_string(), price);
            }
        }

        let names = fresh.column("name")?.str()?;
        let prices = fresh.column("price")?.f64()?;

        let mut prev_price_col: Vec<Option<f64>> = Vec::with_capacity(fresh.height());
        let mut change_col: Vec<Option<f64>> = Vec::with_capacity(fresh.height());
        let mut pct_col: Vec<Option<f64>> = Vec::with_capacity(fresh.height());
        let mut direction_col: Vec<String> = Vec::with_capacity(fresh.height());

        for i in 0..fresh.height() {
            let name = names.get(i).unwrap_or("");
            let price = prices.get(i).unwrap_or(f64::NAN);
            let prev = prev_by_name.get(name).copied();

            prev_price_col.push(prev);

            if let Some(p) = prev {
                if price.is_nan() || p.is_nan() || p == 0.0 {
                    change_col.push(None);
                    pct_col.push(None);
                    direction_col.push(String::new());
                } else {
                    let change = price - p;
                    let pct = (change / p) * 100.0;
                    change_col.push(Some(change));
                    pct_col.push(Some(pct));
                    let direction = if change > 0.01 {
                        "up".to_string()
                    } else if change < -0.01 {
                        "down".to_string()
                    } else {
                        "flat".to_string()
                    };
                    direction_col.push(direction);
                }
            } else {
                change_col.push(None);
                pct_col.push(None);
                direction_col.push(String::new());
            }
        }

        let mut df = fresh;
        df.with_column(Series::new("prev_price".into(), prev_price_col).into())?;
        df.with_column(Series::new("change".into(), change_col).into())?;
        df.with_column(Series::new("pct_change".into(), pct_col).into())?;
        df.with_column(Series::new("direction".into(), direction_col).into())?;

        eprintln!("📊 Updated DataFrame:\n{df}");
        Ok(df)
    }

    fn get_value_by_path(&self, value: &Value, path: &str) -> Option<Value> {
        let mut current = value.clone();

        for part in path.split('.') {
            if part.contains('=') {
                let (field_name, filter_value) = part.split_once('=')?;

                if let Value::Array(arr) = &current {
                    current = arr
                        .iter()
                        .find(|item| {
                            if let Value::Object(map) = item
                                && let Some(field) = map.get(field_name)
                            {
                                return field.as_str().map(|s| s == filter_value).unwrap_or(false);
                            }
                            false
                        })?
                        .clone();
                } else {
                    return None;
                }
            } else {
                current = match &current {
                    Value::Object(map) => map.get(part)?.clone(),
                    Value::Array(arr) => {
                        let index: usize = part.parse().ok()?;
                        arr.get(index)?.clone()
                    }
                    _ => return None,
                };
            }
        }

        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fetcher() -> PriceFetcher {
        PriceFetcher::new().expect("client should build")
    }

    #[test]
    fn max_fetch_attempts_is_three() {
        assert_eq!(MAX_FETCH_ATTEMPTS, 3);
    }

    #[test]
    fn empty_assets_dataframe() {
        let f = fetcher();
        let df = f.fetch_all(&[]).expect("empty df");
        assert_eq!(df.height(), 0);
        assert_eq!(df.width(), 4);
    }

    #[test]
    fn simple_object_path() {
        let f = fetcher();
        let data = json!({"price": 42000.5});
        let v = f.get_value_by_path(&data, "price").unwrap();
        assert_eq!(v.as_f64(), Some(42000.5));
    }

    #[test]
    fn nested_object_path() {
        let f = fetcher();
        let data = json!({
            "data": {
                "quote": {
                    "EUR": {
                        "price": 91.23
                    }
                }
            }
        });
        let v = f.get_value_by_path(&data, "data.quote.EUR.price").unwrap();
        assert_eq!(v.as_f64(), Some(91.23));
    }

    #[test]
    fn array_index_path() {
        let f = fetcher();
        let data = json!([{"price": 10.0}, {"price": 20.0}]);
        let v = f.get_value_by_path(&data, "1.price").unwrap();
        assert_eq!(v.as_f64(), Some(20.0));
    }

    #[test]
    fn array_filter_by_field() {
        let f = fetcher();
        let data = json!({
            "items": [
                {"symbol": "BTC", "price": 50000.0},
                {"symbol": "ETH", "price": 3000.0}
            ]
        });
        let v = f
            .get_value_by_path(&data, "items.symbol=ETH.price")
            .unwrap();
        assert_eq!(v.as_f64(), Some(3000.0));
    }

    #[test]
    fn missing_key_returns_none() {
        let f = fetcher();
        let data = json!({"price": 1.0});
        assert!(f.get_value_by_path(&data, "missing").is_none());
    }
}
