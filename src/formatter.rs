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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== format_price tests =====

    #[test]
    fn test_format_price_nan() {
        assert_eq!(format_price(f64::NAN), "?");
    }

    #[test]
    fn test_format_price_small_number() {
        assert_eq!(format_price(42.50), "42,50");
    }

    #[test]
    fn test_format_price_hundreds() {
        assert_eq!(format_price(500.00), "500,00");
    }

    #[test]
    fn test_format_price_thousands() {
        assert_eq!(format_price(1234.56), "1.234,56");
    }

    #[test]
    fn test_format_price_ten_thousands() {
        assert_eq!(format_price(12345.67), "12.345,67");
    }

    #[test]
    fn test_format_price_hundred_thousands() {
        assert_eq!(format_price(123456.78), "123.456,78");
    }

    #[test]
    fn test_format_price_millions() {
        assert_eq!(format_price(1234567.89), "1.234.567,89");
    }

    #[test]
    fn test_format_price_round_whole_number() {
        assert_eq!(format_price(1000.00), "1.000,00");
    }

    #[test]
    fn test_format_price_zero() {
        assert_eq!(format_price(0.0), "0,00");
    }

    #[test]
    fn test_format_price_single_decimal() {
        assert_eq!(format_price(100.5), "100,50");
    }

    #[test]
    fn test_format_price_large_bitcoin_price() {
        assert_eq!(format_price(68658.00), "68.658,00");
    }

    #[test]
    fn test_format_price_gold_price() {
        assert_eq!(format_price(3814.62), "3.814,62");
    }

    #[test]
    fn test_format_price_gas_price() {
        assert_eq!(format_price(72.46), "72,46");
    }

    // ===== get_symbol tests =====

    #[test]
    fn test_get_symbol_bitcoin() {
        assert_eq!(get_symbol("Bitcoin"), "₿");
    }

    #[test]
    fn test_get_symbol_gold() {
        assert_eq!(get_symbol("Gold"), "🟡");
    }

    #[test]
    fn test_get_symbol_ttf_gas() {
        assert_eq!(get_symbol("TTF Gas"), "🔥");
    }

    #[test]
    fn test_get_symbol_unknown() {
        assert_eq!(get_symbol("Unknown"), "•");
    }

    #[test]
    fn test_get_symbol_empty_string() {
        assert_eq!(get_symbol(""), "•");
    }

    #[test]
    fn test_get_symbol_case_sensitive() {
        assert_eq!(get_symbol("bitcoin"), "•"); // lowercase should not match
    }

    #[test]
    fn test_get_symbol_partial_match() {
        assert_eq!(get_symbol("BTC"), "•"); // partial match should not work
    }
}
