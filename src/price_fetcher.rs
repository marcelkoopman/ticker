use crate::config::Asset;
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;

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
        eprintln!("🔍 Fetching {} from {}", asset.name, asset.url);
        match self.client.get(&asset.url).send() {
            Ok(response) => match response.error_for_status() {
                Ok(resp) => match resp.json::<Value>() {
                    Ok(json) => {
                        eprintln!(
                            "📦 Raw JSON (first 500 chars): {}",
                            serde_json::to_string(&json).unwrap_or_default()[..std::cmp::min(
                                500,
                                serde_json::to_string(&json).unwrap_or_default().len()
                            )]
                                .to_string()
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
                            if let Value::Object(map) = item {
                                if let Some(field) = map.get(field_name) {
                                    return field
                                        .as_str()
                                        .map(|s| s == filter_value)
                                        .unwrap_or(false);
                                }
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
