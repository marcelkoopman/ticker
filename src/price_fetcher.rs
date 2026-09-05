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
        match self.client.get(&asset.url).send() {
            Ok(response) => match response.error_for_status() {
                Ok(resp) => match resp.json::<Value>() {
                    Ok(json) => {
                        if let Some(price_value) = self.get_value_by_path(&json, &asset.price_path)
                        {
                            match price_value {
                                Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
                                Value::String(s) => s.parse().unwrap_or(f64::NAN),
                                _ => f64::NAN,
                            }
                        } else {
                            f64::NAN
                        }
                    }
                    Err(_) => f64::NAN,
                },
                Err(_) => f64::NAN,
            },
            Err(_) => f64::NAN,
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
        let mut current = value;
        for part in path.split('.') {
            current = match current {
                Value::Object(map) => map.get(part)?,
                Value::Array(arr) => {
                    let index: usize = part.parse().ok()?;
                    arr.get(index)?
                }
                _ => return None,
            };
        }
        Some(current.clone())
    }
}
