use polars::prelude::*;
use std::collections::HashMap;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub struct MenuBuilder;

impl MenuBuilder {
    /// Build the tray menu with one menu item per DataFrame row.
    pub fn build(df: &DataFrame, price_history: &HashMap<String, f64>) -> Menu {
        let menu = Menu::new();

        if df.height() == 0 {
            let _ = menu.append(&MenuItem::new("No prices yet", false, None));
        } else {
            let symbols = df.column("symbol").ok().and_then(|c| c.str().ok());
            let names = df.column("name").ok().and_then(|c| c.str().ok());
            let prices = df.column("price").ok().and_then(|c| c.f64().ok());
            let units = df.column("unit").ok().and_then(|c| c.str().ok());

            if let (Some(symbols), Some(names), Some(prices), Some(units)) =
                (symbols, names, prices, units)
            {
                for i in 0..df.height() {
                    let symbol = symbols.get(i).unwrap_or(".");
                    let name = names.get(i).unwrap_or("?");
                    let price = prices.get(i).unwrap_or(f64::NAN);
                    let unit = units.get(i).unwrap_or("");

                    let formatted_price = Self::format_price(price);
                    let currency = Self::unit_to_currency(unit);
                    let prev = price_history.get(name).copied();
                    let change = Self::change_indicator(price, prev, &currency);

                    let row = format!(
                        "{} {} — {} {}{}",
                        symbol, name, currency, formatted_price, change
                    );
                    let item_id = Self::item_id(name);
                    let item = MenuItem::with_id(&item_id, &row, true, None);
                    let _ = menu.append(&item);
                }
            } else {
                let _ = menu.append(&MenuItem::new("Invalid price data", false, None));
            }
        }

        let _ = menu.append(&PredefinedMenuItem::separator());
        let poll_item = MenuItem::with_id("poll", "🔄  Poll now", true, None);
        let _ = menu.append(&poll_item);

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&Self::version_item());
        let quit_item = MenuItem::with_id("quit", " Quit", true, None);
        let _ = menu.append(&quit_item);

        menu
    }

    /// Non-clickable version label (matches Cargo.toml / release tag version).
    pub fn version_item() -> MenuItem {
        MenuItem::new(
            format!("Version {}", env!("CARGO_PKG_VERSION")),
            false,
            None,
        )
    }

    fn item_id(name: &str) -> String {
        name.to_lowercase().replace(' ', "_")
    }

    fn change_indicator(price: f64, prev_price: Option<f64>, currency_symbol: &str) -> String {
        let Some(prev) = prev_price else {
            return String::new();
        };

        if price.is_nan() || prev.is_nan() {
            return String::new();
        }

        let diff = price - prev;
        if diff > 0.01 {
            let change_str = Self::format_price(diff);
            let percent = (diff / prev) * 100.0;
            format!(" 🟢 {} {} (+{:.2}%)", currency_symbol, change_str, percent)
        } else if diff < -0.01 {
            let change_str = Self::format_price(diff.abs());
            let percent = (diff / prev) * 100.0;
            format!(" 🔴 {} {} ({:.2}%)", currency_symbol, change_str, percent)
        } else {
            String::new()
        }
    }

    fn format_price(price: f64) -> String {
        if price.is_nan() {
            return "?".to_string();
        }

        let formatted = format!("{:.2}", price);
        let parts: Vec<&str> = formatted.split('.').collect();

        if parts.len() == 2 {
            let integer_part = parts[0];
            let decimal_part = parts[1];

            let mut result = String::new();
            for (i, ch) in integer_part.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    result.insert(0, '.');
                }
                result.insert(0, ch);
            }

            format!("{},{}", result, decimal_part)
        } else {
            formatted
        }
    }

    fn unit_to_currency(unit: &str) -> String {
        match unit {
            "EUR" => "€".to_string(),
            "USD" => "$".to_string(),
            "GBP" => "£".to_string(),
            "JPY" => "¥".to_string(),
            _ => unit.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_pkg_version() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }

    #[test]
    fn format_price_nan() {
        assert_eq!(MenuBuilder::format_price(f64::NAN), "?");
    }

    #[test]
    fn format_price_thousands() {
        assert_eq!(MenuBuilder::format_price(1234.56), "1.234,56");
    }

    #[test]
    fn build_empty_dataframe_menu() {
        let df = DataFrame::new(vec![
            Series::new("symbol".into(), Vec::<String>::new()).into(),
            Series::new("name".into(), Vec::<String>::new()).into(),
            Series::new("price".into(), Vec::<f64>::new()).into(),
            Series::new("unit".into(), Vec::<String>::new()).into(),
        ])
        .unwrap();
        let _menu = MenuBuilder::build(&df, &HashMap::new());
    }

    #[test]
    fn build_sample_dataframe_menu() {
        let df = DataFrame::new(vec![
            Series::new("symbol".into(), vec!["💰".to_string()]).into(),
            Series::new("name".into(), vec!["Bitcoin".to_string()]).into(),
            Series::new("price".into(), vec![95000.0_f64]).into(),
            Series::new("unit".into(), vec!["EUR".to_string()]).into(),
        ])
        .unwrap();
        let mut history = HashMap::new();
        history.insert("Bitcoin".to_string(), 90000.0);
        let _menu = MenuBuilder::build(&df, &history);
    }

    #[test]
    fn item_id_normalizes_name() {
        assert_eq!(MenuBuilder::item_id("TTF Gas"), "ttf_gas");
    }
}
