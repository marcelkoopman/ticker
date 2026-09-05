pub fn format_price(price: f64) -> String {
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

pub fn get_symbol(name: &str) -> &'static str {
    match name {
        "Bitcoin" => "₿",
        "Gold" => "🟡",
        "TTF Gas" => "🔥",
        _ => "•",
    }
}
