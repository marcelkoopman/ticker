use crate::poller::Poller;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub struct MenuBuilder;

impl MenuBuilder {
    pub fn build(prices: &[(String, f64, String, Option<f64>)], poller: &Poller) -> Menu {
        let menu = Menu::new();

        for (name, price, unit, prev_price) in prices {
            let symbol = match name.as_str() {
                "Bitcoin" => "₿",
                "Gold" => "🟡",
                "TTF Gas" => "🔥",
                _ => "•",
            };

            let formatted_price = Self::format_price(*price);

            // Calculate price change indicator
            let change_indicator = if let Some(prev) = prev_price {
                if price.is_nan() || prev.is_nan() {
                    String::new()
                } else {
                    let diff = price - prev;
                    if diff > 0.01 {
                        let change_str = Self::format_price(diff);
                        format!(" ↑ {}", change_str)
                    } else if diff < -0.01 {
                        let change_str = Self::format_price(diff.abs());
                        format!(" ↓ {}", change_str)
                    } else {
                        " ↔".to_string()
                    }
                }
            } else {
                String::new()
            };

            // Show timestamp
            let poll_display = poller
                .next_poll_in(name)
                .map(|s| format!("({})", s))
                .unwrap_or_default();

            let row = format!(
                "{} {} — {} {}{} {}",
                symbol, name, formatted_price, unit, change_indicator, poll_display
            );

            let item_id = name.to_lowercase().replace(" ", "_");
            let item = MenuItem::with_id(&item_id, &row, true, None);
            let _ = menu.append(&item);
        }

        let _ = menu.append(&PredefinedMenuItem::separator());
        let poll_item = MenuItem::with_id("poll", "🔄  Poll now", true, None);
        let _ = menu.append(&poll_item);

        let _ = menu.append(&PredefinedMenuItem::separator());
        let quit_item = MenuItem::with_id("quit", " Quit", true, None);
        let _ = menu.append(&quit_item);

        menu
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
}
