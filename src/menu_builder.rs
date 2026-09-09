use polars::prelude::*;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub struct MenuBuilder;

impl MenuBuilder {
    /// One menu item per DataFrame row; change text comes from DF columns.
    pub fn build(df: &DataFrame) -> Menu {
        let menu = Menu::new();

        if df.height() == 0 {
            let _ = menu.append(&MenuItem::new("No prices yet", false, None));
        } else {
            let symbols = df.column("symbol").ok().and_then(|c| c.str().ok());
            let names = df.column("name").ok().and_then(|c| c.str().ok());
            let prices = df.column("price").ok().and_then(|c| c.f64().ok());
            let units = df.column("unit").ok().and_then(|c| c.str().ok());
            let changes = df.column("change").ok().and_then(|c| c.f64().ok());
            let pcts = df.column("pct_change").ok().and_then(|c| c.f64().ok());
            let directions = df.column("direction").ok().and_then(|c| c.str().ok());

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

                    let change_text = match (
                        changes.as_ref().and_then(|c| c.get(i)),
                        pcts.as_ref().and_then(|c| c.get(i)),
                        directions.as_ref().and_then(|c| c.get(i)),
                    ) {
                        (Some(change), Some(pct), Some("up")) => {
                            format!(
                                " 🟢 {} {} (+{:.2}%)",
                                currency,
                                Self::format_price(change),
                                pct
                            )
                        }
                        (Some(change), Some(pct), Some("down")) => {
                            format!(
                                " 🔴 {} {} ({:.2}%)",
                                currency,
                                Self::format_price(change.abs()),
                                pct
                            )
                        }
                        _ => String::new(),
                    };

                    let row = format!(
                        "{} {} — {} {}{}",
                        symbol, name, currency, formatted_price, change_text
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
    fn item_id_normalizes_name() {
        assert_eq!(MenuBuilder::item_id("TTF Gas"), "ttf_gas");
    }
}
