use crate::models::{Asset, Price};
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;

fn get_value_by_path(value: &Value, path: &str) -> Option<Value> {
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

pub fn fetch_prices(assets: &[Asset]) -> Result<Vec<Price>, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent("rust-price-fetcher/1.0")
        .build()?;

    let mut results = Vec::new();

    for asset in assets {
        let price_value = match client.get(&asset.url).send() {
            Ok(response) => match response.error_for_status() {
                Ok(resp) => match resp.json::<Value>() {
                    Ok(json) => get_value_by_path(&json, &asset.price_path),
                    Err(_) => None,
                },
                Err(_) => None,
            },
            Err(_) => None,
        };

        let value = match price_value {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
            Some(Value::String(s)) => s.parse().unwrap_or(f64::NAN),
            _ => f64::NAN,
        };

        results.push(Price {
            name: asset.name.clone(),
            value,
            unit: asset.unit.clone(),
        });
    }

    Ok(results)
}
