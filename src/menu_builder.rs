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
    ///
    /// Price rows use a short two-line layout so the popover stays narrow:
    /// ```text
    /// 💰 Bitcoin  €66.672 / BTC
    ///             +€217 · +0,33%
    /// ```
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
                    let symbol = symbols.get(i).unwrap_or("");
                    let name = names.get(i).unwrap_or("?");
                    let price = prices.get(i).unwrap_or(f64::NAN);
                    let unit = units.get(i).unwrap_or("");
                    let unit_hint = unit_hints.get(i).unwrap_or("");

                    let row = Self::format_price_row(
                        symbol,
                        name,
                        price,
                        unit,
                        unit_hint,
                        changes.as_ref().and_then(|c| c.get(i)),
                        pcts.as_ref().and_then(|c| c.get(i)),
                        directions.as_ref().and_then(|c| c.get(i)),
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
            let mark = if watch.triggered { "✓" } else { " " };
            let item_text = format!(
                "{} {} {}  €{}",
                mark,
                watch.direction.emoji(),
                watch.asset_name,
                Self::format_price(watch.target_price)
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

    /// Compact two-line price row (primary price + day change).
    fn format_price_row(
        symbol: &str,
        name: &str,
        price: f64,
        unit: &str,
        unit_hint: &str,
        change: Option<f64>,
        pct: Option<f64>,
        direction: Option<&str>,
    ) -> String {
        let currency = Self::unit_to_currency(unit);
        let price_txt = Self::format_price(price);

        // unit_hint is often "/BTC", "/MWh", etc.
        let unit_part = {
            let h = unit_hint.trim();
            if h.is_empty() {
                String::new()
            } else if h.starts_with('/') {
                format!(" {}", h)
            } else {
                format!(" / {}", h)
            }
        };

        let label = if symbol.is_empty() {
            name.to_string()
        } else {
            format!("{} {}", symbol, name)
        };

        // Line 1: "💰 Bitcoin  €66.672 / BTC"
        let line1 = format!("{}  {}{}{}", label, currency, price_txt, unit_part);

        // Line 2: "  +€217 · +0,33%"  (indented; empty when no change data)
        let line2 = match (change, pct, direction) {
            (Some(c), Some(p), Some("up")) => format!(
                "  +{}{} · +{:.2}%",
                currency,
                Self::format_price(c),
                p
            ),
            (Some(c), Some(p), Some("down")) => format!(
                "  −{}{} · {:.2}%",
                currency,
                Self::format_price(c.abs()),
                p
            ),
            (Some(c), Some(p), Some("flat")) => format!(
                "  {}{} · {:.2}%",
                currency,
                Self::format_price(c.abs()),
                p
            ),
            _ => String::new(),
        };

        if line2.is_empty() {
            line1
        } else {
            format!("{line1}\n{line2}")
        }
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
    fn format_price_row_two_lines_up() {
        let row = MenuBuilder::format_price_row(
            "💰",
            "Bitcoin",
            66672.0,
            "EUR",
            "/BTC",
            Some(217.0),
            Some(0.33),
            Some("up"),
        );
        assert!(row.contains('\n'));
        let lines: Vec<_> = row.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Bitcoin"));
        assert!(lines[0].contains("66.672"));
        assert!(lines[1].contains("+€"));
        assert!(lines[1].contains("+0.33%"));
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
