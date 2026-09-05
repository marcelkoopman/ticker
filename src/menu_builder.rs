use crate::formatter::{format_price, get_symbol};
use crate::models::Price;
use crate::price_tracker;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub fn build_menu_with_next_poll(prices: &[Price], next_poll_str: &str, last_update: &str) -> Menu {
    let menu = Menu::new();

    let pinned_prices = price_tracker::load_pinned_prices().unwrap_or_default();

    for price in prices {
        let symbol = get_symbol(&price.name);
        let formatted = format_price(price.value);

        let change_indicator =
            price_tracker::get_price_change(&price.name, price.value).unwrap_or_default();

        let is_pinned = pinned_prices.contains_key(&price.name);

        let row = if change_indicator.is_empty() {
            format!("{} {} — {} {}", symbol, price.name, formatted, price.unit)
        } else {
            format!(
                "{} {} — {} {} {}",
                symbol, price.name, formatted, price.unit, change_indicator
            )
        };

        let item_id = price.name.to_lowercase().replace(" ", "_");
        let item = MenuItem::with_id(&item_id, &row, true, None);
        let _ = menu.append(&item);

        let pin_label = if is_pinned {
            format!("  📌 Unpin {}", price.name)
        } else {
            format!("  📍 Pin {}", price.name)
        };

        let pin_id = format!("pin_{}", price.name.to_lowercase().replace(" ", "_"));
        let pin_item = MenuItem::with_id(&pin_id, &pin_label, true, None);
        let _ = menu.append(&pin_item);

        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    let timestamp_label = format!("⏰ Last update: {}", last_update);
    let timestamp_item = MenuItem::new(&timestamp_label, false, None);
    let _ = menu.append(&timestamp_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    let poll_label = format!("🔄 Next poll at {}", next_poll_str);
    let poll_item = MenuItem::with_id("poll", &poll_label, true, None);
    let _ = menu.append(&poll_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    let quit_item = MenuItem::with_id("quit", " Quit", true, None);
    let _ = menu.append(&quit_item);

    menu
}
