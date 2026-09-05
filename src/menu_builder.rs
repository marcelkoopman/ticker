use crate::formatter::{format_price, get_symbol};
use crate::models::Price;
use crate::price_history;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub fn build_menu_with_next_poll(prices: &[Price], next_poll_str: &str, last_update: &str) -> Menu {
    let menu = Menu::new();

    // Price items with delta indicators
    for price in prices {
        let symbol = get_symbol(&price.name);
        let formatted = format_price(price.value);

        // Calculate delta from previous poll
        let delta =
            price_history::get_delta(&price.name, price.value).unwrap_or_else(|_| "⚪".to_string());

        let row = if delta == "⚪" {
            format!("{} {} — {} {}", symbol, price.name, formatted, price.unit)
        } else {
            format!(
                "{} {} — {} {} {}",
                symbol, price.name, formatted, price.unit, delta
            )
        };

        let item_id = price.name.to_lowercase().replace(" ", "_");
        let item = MenuItem::with_id(&item_id, &row, true, None);
        let _ = menu.append(&item);
    }

    let _ = menu.append(&PredefinedMenuItem::separator());

    // Last update timestamp
    let timestamp_label = format!("⏰ Last update: {}", last_update);
    let timestamp_item = MenuItem::new(&timestamp_label, false, None);
    let _ = menu.append(&timestamp_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    // Next poll time
    let poll_label = format!("🔄 Next poll at {}", next_poll_str);
    let poll_item = MenuItem::with_id("poll", &poll_label, true, None);
    let _ = menu.append(&poll_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    // Version info
    let version_label = format!("ℹ Version v{}", env!("CARGO_PKG_VERSION"));
    let version_item = MenuItem::new(&version_label, false, None);
    let _ = menu.append(&version_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    // Quit
    let quit_item = MenuItem::with_id("quit", " Quit", true, None);
    let _ = menu.append(&quit_item);

    menu
}

/// Alternative: Build menu without showing next poll time.
pub fn build_menu_minimal(prices: &[Price], last_update: &str) -> Menu {
    let menu = Menu::new();

    for price in prices {
        let symbol = get_symbol(&price.name);
        let formatted = format_price(price.value);

        let delta =
            price_history::get_delta(&price.name, price.value).unwrap_or_else(|_| "⚪".to_string());

        let row = if delta == "⚪" {
            format!("{} {} — {} {}", symbol, price.name, formatted, price.unit)
        } else {
            format!(
                "{} {} — {} {} {}",
                symbol, price.name, formatted, price.unit, delta
            )
        };

        let item_id = price.name.to_lowercase().replace(" ", "_");
        let item = MenuItem::with_id(&item_id, &row, true, None);
        let _ = menu.append(&item);
    }

    let _ = menu.append(&PredefinedMenuItem::separator());

    let timestamp_label = format!("⏰ {}", last_update);
    let timestamp_item = MenuItem::new(&timestamp_label, false, None);
    let _ = menu.append(&timestamp_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    let poll_item = MenuItem::with_id("poll", "🔄 Refresh now", true, None);
    let _ = menu.append(&poll_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    let quit_item = MenuItem::with_id("quit", " Quit", true, None);
    let _ = menu.append(&quit_item);

    menu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_menu_minimal_with_prices() {
        let prices = vec![
            Price {
                name: "Bitcoin".to_string(),
                value: 50_000.0,
                unit: "EUR".to_string(),
            },
            Price {
                name: "Gold".to_string(),
                value: 3_800.0,
                unit: "EUR".to_string(),
            },
        ];

        let _menu = build_menu_minimal(&prices, "14:30:00");
    }

    #[test]
    fn test_build_menu_minimal_empty() {
        let prices: Vec<Price> = vec![];
        let _menu = build_menu_minimal(&prices, "14:30:00");
    }

    #[test]
    fn test_build_menu_with_next_poll_single_price() {
        let prices = vec![Price {
            name: "TTF Gas".to_string(),
            value: 72.5,
            unit: "EUR/MWh".to_string(),
        }];

        let _menu = build_menu_with_next_poll(&prices, "14:31:30", "14:30:00");
    }

    #[test]
    fn test_build_menu_with_next_poll_multiple_prices() {
        let prices = vec![
            Price {
                name: "Bitcoin".to_string(),
                value: 68_877.0,
                unit: "EUR".to_string(),
            },
            Price {
                name: "Gold".to_string(),
                value: 3_814.62,
                unit: "EUR".to_string(),
            },
            Price {
                name: "TTF Gas".to_string(),
                value: 71.95,
                unit: "EUR/MWh".to_string(),
            },
        ];

        let _menu = build_menu_with_next_poll(&prices, "14:31:30", "14:30:00");
    }

    #[test]
    fn test_build_menu_empty_prices() {
        let prices: Vec<Price> = vec![];
        let _menu = build_menu_with_next_poll(&prices, "14:31:30", "14:30:00");
    }
}
