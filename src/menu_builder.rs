use crate::formatter::{format_price, get_symbol};
use crate::models::Price;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub fn build_menu(prices: &[Price], timestamp: &str) -> Menu {
    let menu = Menu::new();

    for price in prices {
        let symbol = get_symbol(&price.name);
        let formatted = format_price(price.value);
        let row = format!("{} {} — {} {}", symbol, price.name, formatted, price.unit);
        let item_id = price.name.to_lowercase().replace(" ", "_");

        let item = MenuItem::with_id(&item_id, &row, true, None);
        let _ = menu.append(&item);
    }

    let _ = menu.append(&PredefinedMenuItem::separator());

    let poll_label = format!("🔄  Poll again ({})", timestamp);
    let poll_item = MenuItem::with_id("poll", &poll_label, true, None);
    let _ = menu.append(&poll_item);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let quit_item = MenuItem::with_id("quit", " Quit", true, None);
    let _ = menu.append(&quit_item);

    menu
}
