use polars::prelude::*;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

use crate::price_watch::WatchList;
use crate::watch_ui::WatchUIBuilder;

/// Keep in sync with the `polars` version in Cargo.toml.
const POLARS_VERSION: &str = "0.55.2";

pub struct MenuBuilder;

impl MenuBuilder {
    /// One menu item per DataFrame row; change text comes from DF columns.
    /// Also renders the price-watch section.
    pub fn build(df: &DataFrame, watch_list: &WatchList) -> Menu {
        let menu = Menu::new();

        if df.height() == 0 {
            let _ = menu.append(&MenuItem::new("No prices yet", false, None));
        } else {
            let symbols = df.column("symbol").ok().and_then(|c| c.str().ok());
            let names = df.column("name").ok().and_then(|c| c.str().ok());
            let prices = df.column("price").ok().and_then(|c| c.f64().ok());
            let units = df.column("unit").ok().and_then(|c| c.str().ok());
            let unit_hints = df.column("unit_hint").ok().and_then(|c| c.str().ok());

            // Day change is the primary signal shown in the menu.
            let changes = df.column("change_day").ok().and_then(|c| c.f64().ok());
            let pcts = df.column("pct_day").ok().and_then(|c| c.f64().ok());
            let directions = df.column("direction_day").ok().and_then(|c| c.str().ok());

            if let (Some(symbols), Some(names), Some(prices), Some(units), Some(unit_hints)) =
                (symbols, names, prices, units, unit_hints)
            {
                for i in 0..df.height() {
                    let symbol = symbols.get(i).unwrap_or(".");
                    let name = names.get(i).unwrap_or("?");
                    let price = prices.get(i).unwrap_or(f64::NAN);
                    let unit = units.get(i).unwrap_or("");
                    let unit_hint = unit_hints.get(i).unwrap_or("");

                    let formatted_price = Self::format_price(price);
                    let currency = Self::unit_to_currency(unit);

                    let change_text = match (
                        changes.as_ref().and_then(|c| c.get(i)),
                        pcts.as_ref().and_then(|c| c.get(i)),
                        directions.as_ref().and_then(|c| c.get(i)),
                    ) {
                        (Some(change), Some(pct), Some("up")) => {
                            format!(
                                " 🟢 today {} {} (+{:.2}%)",
                                currency,
                                Self::format_price(change),
                                pct
                            )
                        }
                        (Some(change), Some(pct), Some("down")) => {
                            format!(
                                " 🔴 today {} {} ({:.2}%)",
                                currency,
                                Self::format_price(change.abs()),
                                pct
                            )
                        }
                        (Some(change), Some(pct), Some("flat")) => {
                            format!(
                                " | today {} {} ({:.2}%)",
                                currency,
                                Self::format_price(change.abs()),
                                pct
                            )
                        }
                        _ => String::new(),
                    };

                    let row = format!(
                        "{} {} - {}{} {}{}",
                        symbol, name, currency, formatted_price, unit_hint, change_text
                    );
                    let item_id = Self::item_id(name);
                    let item = MenuItem::with_id(&item_id, &row, true, None);
                    let _ = menu.append(&item);
                }
            } else {
                let _ = menu.append(&MenuItem::new("Invalid price data", false, None));
            }
        }

        // --- PRICE WATCH SECTION ---
        let _ = menu.append(&PredefinedMenuItem::separator());

        let status_title = WatchUIBuilder::watch_status_indicator(watch_list);
        let _ = menu.append(&MenuItem::new(&status_title, false, None));

        for watch in &watch_list.watches {
            let status = if watch.triggered { "✓" } else { " " };
            let direction_emoji = watch.direction.emoji();
            let item_text = format!(
                "  [{}] {} {} - €{:.2}",
                status, direction_emoji, watch.asset_name, watch.target_price
            );
            let item_id = format!(
                "watch_{}_{}",
                watch.asset_name.to_lowercase().replace(' ', "_"),
                watch.target_price
            );
            let _ = menu.append(&MenuItem::with_id(&item_id, &item_text, true, None));
        }

        let _ = menu.append(&MenuItem::with_id(
            "add_watch",
            "➕ Add Price Watch",
            true,
            None,
        ));
        let _ = menu.append(&MenuItem::with_id(
            "manage_watches",
            "⚙️ Manage Watches",
            true,
            None,
        ));
        // --------------------------

        let _ = menu.append(&PredefinedMenuItem::separator());
        let poll_item = MenuItem::with_id("poll", "🔄  Poll now", true, None);
        let _ = menu.append(&poll_item);
        let copy_item = MenuItem::with_id("copy", "📋  Copy to clipboard", true, None);
        let _ = menu.append(&copy_item);

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&Self::version_item());
        let quit_item = MenuItem::with_id("quit", " Quit", true, None);
        let _ = menu.append(&quit_item);

        menu
    }

    /// Tab-separated table for paste into Numbers / Excel / editors.
    pub fn dataframe_as_tsv(df: &DataFrame) -> String {
        let cols = [
            "symbol",
            "name",
            "price",
            "unit",
            "unit_hint",
            "day_open",
            "change_day",
            "pct_day",
            "direction_day",
        ];

        let mut out = String::new();
        out.push_str(&cols.join("\t"));
        out.push('\n');

        if df.height() == 0 {
            return out;
        }

        for i in 0..df.height() {
            let mut cells = Vec::with_capacity(cols.len());
            for col_name in &cols {
                let cell = match df.column(col_name) {
                    Ok(col) => Self::cell_at(col, i),
                    Err(_) => String::new(),
                };
                cells.push(cell);
            }
            out.push_str(&cells.join("\t"));
            out.push('\n');
        }
        out
    }

    fn cell_at(col: &Column, row: usize) -> String {
        if let Ok(ca) = col.str()
            && let Some(s) = ca.get(row)
        {
            return s.to_string();
        }
        if let Ok(ca) = col.f64() {
            return match ca.get(row) {
                Some(v) if v.is_nan() => String::new(),
                Some(v) => format!("{:.6}", v),
                None => String::new(),
            };
        }
        String::new()
    }

    pub fn version_item() -> MenuItem {
        MenuItem::new(
            format!(
                "Version {} · Polars {}",
                env!("CARGO_PKG_VERSION"),
                POLARS_VERSION
            ),
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
    fn version_includes_app_and_polars() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
        assert!(!POLARS_VERSION.is_empty());
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

    #[test]
    fn dataframe_as_tsv_empty_has_header() {
        let df = DataFrame::new_infer_height(vec![
            Series::new("symbol".into(), Vec::<String>::new()).into(),
            Series::new("name".into(), Vec::<String>::new()).into(),
            Series::new("price".into(), Vec::<f64>::new()).into(),
            Series::new("unit".into(), Vec::<String>::new()).into(),
            Series::new("unit_hint".into(), Vec::<String>::new()).into(),
            Series::new("day_open".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("change_day".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("pct_day".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("direction_day".into(), Vec::<String>::new()).into(),
        ])
        .expect("empty df");
        let tsv = MenuBuilder::dataframe_as_tsv(&df);
        assert!(tsv.starts_with("symbol\tname\tprice"));
        assert_eq!(tsv.lines().count(), 1);
    }
}
