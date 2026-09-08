use crate::poller::Poller;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub struct MenuBuilder;

impl MenuBuilder {
    pub fn build(prices: &[(String, f64, String, Option<f64>, String)], _poller: &Poller) -> Menu {
        let menu = Menu::new();

        for (name, price, unit, prev_price, symbol) in prices {
            let formatted_price = Self::format_price(*price);
            let currency_symbol = Self::unit_to_currency(unit);
            let change_indicator = Self::change_indicator(*price, *prev_price, &currency_symbol);

            let row = format!(
                "{} {} — {} {}{}",
                symbol, name, currency_symbol, formatted_price, change_indicator
            );

            let item_id = Self::item_id(name);
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
    fn format_price_nan() {
        assert_eq!(MenuBuilder::format_price(f64::NAN), "?");
    }

    #[test]
    fn format_price_simple() {
        assert_eq!(MenuBuilder::format_price(12.34), "12,34");
        assert_eq!(MenuBuilder::format_price(0.5), "0,50");
        assert_eq!(MenuBuilder::format_price(100.0), "100,00");
    }

    #[test]
    fn format_price_thousands_separator() {
        assert_eq!(MenuBuilder::format_price(1234.56), "1.234,56");
        assert_eq!(MenuBuilder::format_price(1234567.89), "1.234.567,89");
        assert_eq!(MenuBuilder::format_price(1000.0), "1.000,00");
    }

    #[test]
    fn format_price_negative() {
        let s = MenuBuilder::format_price(-42.5);
        assert!(s.contains("42,50") || s.contains("-42,50"));
    }

    #[test]
    fn unit_to_currency_known() {
        assert_eq!(MenuBuilder::unit_to_currency("EUR"), "€");
        assert_eq!(MenuBuilder::unit_to_currency("USD"), "$");
        assert_eq!(MenuBuilder::unit_to_currency("GBP"), "£");
        assert_eq!(MenuBuilder::unit_to_currency("JPY"), "¥");
    }

    #[test]
    fn unit_to_currency_unknown_passthrough() {
        assert_eq!(MenuBuilder::unit_to_currency("EUR/MWh"), "EUR/MWh");
        assert_eq!(MenuBuilder::unit_to_currency("BTC"), "BTC");
    }

    #[test]
    fn item_id_normalizes_name() {
        assert_eq!(MenuBuilder::item_id("Bitcoin"), "bitcoin");
        assert_eq!(MenuBuilder::item_id("TTF Gas"), "ttf_gas");
        assert_eq!(MenuBuilder::item_id("ETH"), "eth");
    }

    #[test]
    fn change_indicator_none_when_no_prev() {
        assert_eq!(MenuBuilder::change_indicator(100.0, None, "€"), "");
    }

    #[test]
    fn change_indicator_none_when_nan() {
        assert_eq!(
            MenuBuilder::change_indicator(f64::NAN, Some(100.0), "€"),
            ""
        );
        assert_eq!(
            MenuBuilder::change_indicator(100.0, Some(f64::NAN), "€"),
            ""
        );
    }

    #[test]
    fn change_indicator_none_when_unchanged() {
        assert_eq!(MenuBuilder::change_indicator(100.0, Some(100.005), "€"), "");
    }

    #[test]
    fn change_indicator_up() {
        let s = MenuBuilder::change_indicator(110.0, Some(100.0), "€");
        assert!(s.contains("🟢"));
        assert!(s.contains("€"));
        assert!(s.contains("+10.00%"));
    }

    #[test]
    fn change_indicator_down() {
        let s = MenuBuilder::change_indicator(90.0, Some(100.0), "€");
        assert!(s.contains("🔴"));
        assert!(s.contains("€"));
        assert!(s.contains("-10.00%"));
    }
}
