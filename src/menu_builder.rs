use polars::prelude::*;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use crate::price_watch::WatchList;
use crate::watch_ui::WatchUIBuilder;

/// Keep in sync with the `polars` version in Cargo.toml.
const POLARS_VERSION: &str = "0.55.2";

pub struct MenuBuilder;

impl MenuBuilder {
    /// One menu item per DataFrame row; change text comes from DF columns.
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

        // --- PRICE WATCH SECTIE ---
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

        let _ = menu.append(&MenuItem::with_id("add_watch", "➕ Add Price Watch", true, None));
        let _ = menu.append(&MenuItem::with_id("manage_watches", "⚙️ Manage Watches", true, None));
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

    fn item_id(name: &str) -> String {
        name.to_lowercase().replace(' ', "_")
    }

    fn format_price(val: f64) -> String {
        if val.is_nan() {
            return "N/A".to_string();
        }
        let formatted = format!("{:.2}", val);
        let parts: Vec<&str> = formatted.split('.').collect();
        let int_part = parts[0];
        let dec_part = parts.get(1).copied().unwrap_or("00");

        let formatted_int = int_part
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<_>>()
            .join(",");

        format!("{}.{}", formatted_int, dec_part)
    }

    fn unit_to_currency(unit: &str) -> &'static str {
        match unit.to_uppercase().as_str() {
            "EUR" => "€",
            "USD" => "$",
            "GBP" => "£",
            _ => "",
        }
    }

    fn version_item() -> MenuItem {
        let text = format!("Polars v{}", POLARS_VERSION);
        MenuItem::new(&text, false, None)
    }
}
