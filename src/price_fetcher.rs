use crate::config::Asset;
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

    /// Fetch a price, retrying up to 3 times when the result is NaN (network/HTTP/parse failure).
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
                            eprintln!(
                                "✓ Found value at path '{}': {:?}",
                                asset.price_path, price_value
                            );
                            match price_value {
                                Value::Number(n) => {
                                    let price = n.as_f64().unwrap_or(f64::NAN);
                                    eprintln!("  Parsed as: {}", price);
                                    price
                                }
                                Value::String(s) => {
                                    let price = s.parse().unwrap_or(f64::NAN);
                                    eprintln!("  Parsed string as: {}", price);
                                    price
                                }
                                _ => {
                                    eprintln!("  Unexpected type: {:?}", price_value);
                                    f64::NAN
                                }
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

    pub fn fetch_all(&self, assets: &[Asset]) -> Vec<(String, f64, String)> {
        assets
            .iter()
            .map(|asset| {
                let price = self.fetch_price(asset);
                (asset.name.clone(), price, asset.unit.clone())
            })
            .collect()
    }

    fn get_value_by_path(&self, value: &Value, path: &str) -> Option<Value> {
        let mut current = value.clone(); // Work with owned Value

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
    fn array_filter_not_found() {
        let f = fetcher();
        let data = json!({
            "items": [
                {"symbol": "BTC", "price": 50000.0}
            ]
        });
        assert!(
            f.get_value_by_path(&data, "items.symbol=ETH.price")
                .is_none()
        );
    }

    #[test]
    fn missing_key_returns_none() {
        let f = fetcher();
        let data = json!({"price": 1.0});
        assert!(f.get_value_by_path(&data, "missing").is_none());
        assert!(f.get_value_by_path(&data, "price.nested").is_none());
    }

    #[test]
    fn string_value() {
        let f = fetcher();
        let data = json!({"price": "1234.56"});
        let v = f.get_value_by_path(&data, "price").unwrap();
        assert_eq!(v.as_str(), Some("1234.56"));
    }

    #[test]
    fn invalid_array_index() {
        let f = fetcher();
        let data = json!([1, 2, 3]);
        assert!(f.get_value_by_path(&data, "5").is_none());
        assert!(f.get_value_by_path(&data, "abc").is_none());
    }

    #[test]
    fn filter_on_non_array_returns_none() {
        let f = fetcher();
        let data = json!({"symbol": "BTC"});
        assert!(f.get_value_by_path(&data, "symbol=BTC").is_none());
    }
}
