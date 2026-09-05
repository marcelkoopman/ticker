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

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn fetch_prices(assets: &[Asset]) -> Result<Vec<Price>, Box<dyn Error>> {
    eprintln!("🔗 Building HTTP client...");

    let client = Client::builder()
        .user_agent("rust-price-fetcher/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut results = Vec::new();

    eprintln!("📥 Fetching {} assets...", assets.len());

    for asset in assets {
        eprintln!("\n🎯 Processing asset: {}", asset.name);
        eprintln!("   URL: {}", asset.url);
        eprintln!("   Path: {}", asset.price_path);

        let price = match client.get(&asset.url).send() {
            Ok(response) => {
                eprintln!("   ✓ HTTP response received (status: {})", response.status());

                match response.error_for_status() {
                    Ok(resp) => {
                        eprintln!("   ✓ Status OK");

                        match resp.text() {
                            Ok(text) => {
                                eprintln!("   📄 Response body (first 500 chars):");
                                let preview = if text.len() > 500 {
                                    format!("{}...", &text[..500])
                                } else {
                                    text.clone()
                                };
                                eprintln!("      {}", preview);

                                match serde_json::from_str::<Value>(&text) {
                                    Ok(json) => {
                                        eprintln!("   ✓ JSON parsed successfully");
                                        eprintln!("   🔍 Full JSON: {}", json);

                                        if let Some(price_value) = get_value_by_path(&json, &asset.price_path) {
                                            eprintln!("   ✓ Value found at path: {}", price_value);

                                            match price_value {
                                                Value::Number(n) => {
                                                    let price = n.as_f64().unwrap_or(f64::NAN);
                                                    eprintln!("   ✓ Parsed as number: {}", price);
                                                    price
                                                }
                                                Value::String(s) => {
                                                    eprintln!("   🔤 String value: {}", s);
                                                    match s.parse::<f64>() {
                                                        Ok(price) => {
                                                            eprintln!("   ✓ Parsed string as number: {}", price);
                                                            price
                                                        }
                                                        Err(e) => {
                                                            eprintln!("   ❌ Failed to parse string: {}", e);
                                                            f64::NAN
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    eprintln!(
                                                        "   ❌ Wrong type: expected number or string, got: {}",
                                                        value_type_name(&price_value)
                                                    );
                                                    f64::NAN
                                                }
                                            }
                                        } else {
                                            eprintln!("   ❌ Path '{}' not found in JSON", asset.price_path);
                                            if let Some(obj) = json.as_object() {
                                                eprintln!("   Available keys: {:?}", obj.keys().collect::<Vec<_>>());
                                            }
                                            f64::NAN
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("   ❌ JSON parse error: {}", e);
                                        f64::NAN
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("   ❌ Failed to read response body: {}", e);
                                f64::NAN
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("   ❌ HTTP error status: {}", e);
                        f64::NAN
                    }
                }
            }
            Err(e) => {
                eprintln!("   ❌ Request failed: {}", e);
                f64::NAN
            }
        };

        eprintln!("   📊 Final price for {}: {}", asset.name, price);
        results.push(Price {
            name: asset.name.clone(),
            value: price,
            unit: asset.unit.clone(),
        });
    }

    eprintln!("\n✅ Fetch complete. Got {} prices", results.len());
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_value_by_path_simple() {
        let json = serde_json::json!({"price": 100.0});
        let value = get_value_by_path(&json, "price");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), 100.0);
    }

    #[test]
    fn test_get_value_by_path_nested() {
        let json = serde_json::json!({"bpi": {"EUR": {"rate_float": 50000.0}}});
        let value = get_value_by_path(&json, "bpi.EUR.rate_float");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), 50000.0);
    }

    #[test]
    fn test_get_value_by_path_array() {
        let json = serde_json::json!([{"value": 100}, {"value": 200}]);
        let value = get_value_by_path(&json, "0.value");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), 100);
    }

    #[test]
    fn test_get_value_by_path_not_found() {
        let json = serde_json::json!({"price": 100.0});
        let value = get_value_by_path(&json, "nonexistent");
        assert!(value.is_none());
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(value_type_name(&Value::Null), "null");
        assert_eq!(value_type_name(&Value::Bool(true)), "bool");
        assert_eq!(value_type_name(&serde_json::json!(1.0)), "number");
        assert_eq!(value_type_name(&Value::String("test".to_string())), "string");
        assert_eq!(value_type_name(&Value::Array(vec![])), "array");
        assert_eq!(value_type_name(&Value::Object(Default::default())), "object");
    }
}
