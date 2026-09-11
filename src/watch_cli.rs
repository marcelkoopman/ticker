use crate::price_watch::{load_watch_list, save_watch_list, WatchDirection, WatchList};
use std::error::Error;

pub fn handle_watch_command(args: &[String]) -> Result<String, Box<dyn Error>> {
    if args.is_empty() {
        return Err("No command specified".into());
    }

    match args[0].as_str() {
        "add" => add_watch(&args[1..]),
        "remove" => remove_watch(&args[1..]),
        "list" => list_watches(),
        "clear" => clear_watches(),
        "reset" => reset_triggered(),
        "help" | "--help" | "-h" => Ok(get_help_text()),
        cmd => Err(format!("Unknown command: {}", cmd).into()),
    }
}

fn add_watch(args: &[String]) -> Result<String, Box<dyn Error>> {
    if args.len() < 3 {
        return Err(
            "Usage: ticker add <asset_name> <target_price> <above|below>".into(),
        );
    }

    let asset_name = args[0].clone();
    let target_price: f64 = args[1].parse()?;
    let direction = match args[2].to_lowercase().as_str() {
        "above" => WatchDirection::Above,
        "below" => WatchDirection::Below,
        _ => return Err("Direction must be 'above' or 'below'".into()),
    };

    let mut watch_list = load_watch_list()?;

    // Check if watch already exists
    if watch_list
        .watches
        .iter()
        .any(|w| w.asset_name == asset_name && (w.target_price - target_price).abs() < 0.01)
    {
        return Err(format!(
            "Watch already exists for {} at €{:.2}",
            asset_name, target_price
        )
        .into());
    }

    watch_list.add_watch(asset_name.clone(), target_price, direction.clone());
    save_watch_list(&watch_list)?;

    Ok(format!(
        "✅ Added watch: {} {} €{:.2}",
        direction.emoji(),
        asset_name,
        target_price
    ))
}

fn remove_watch(args: &[String]) -> Result<String, Box<dyn Error>> {
    if args.len() < 2 {
        return Err(
            "Usage: ticker remove <asset_name> <target_price>".into(),
        );
    }

    let asset_name = &args[0];
    let target_price: f64 = args[1].parse()?;

    let mut watch_list = load_watch_list()?;

    if watch_list.remove_watch(asset_name, target_price) {
        save_watch_list(&watch_list)?;
        Ok(format!(
            "✅ Removed watch for {} at €{:.2}",
            asset_name, target_price
        ))
    } else {
        Err(format!(
            "❌ Watch not found for {} at €{:.2}",
            asset_name, target_price
        )
        .into())
    }
}

fn list_watches() -> Result<String, Box<dyn Error>> {
    let watch_list = load_watch_list()?;

    if watch_list.watches.is_empty() {
        return Ok("📭 No price watches configured".to_string());
    }

    let mut output = String::from("📋 Price Watches:\n\n");

    for (i, watch) in watch_list.watches.iter().enumerate() {
        let status = if watch.triggered { "✓" } else { " " };
        let direction_emoji = watch.direction.emoji();
        let direction_text = watch.direction.as_str();
        output.push_str(&format!(
            "{}. [{}] {} {} - €{:.2} ({})
",
            i + 1,
            status,
            direction_emoji,
            watch.asset_name,
            watch.target_price,
            direction_text
        ));
    }

    Ok(output)
}

fn clear_watches() -> Result<String, Box<dyn Error>> {
    let watch_list = WatchList::new();
    save_watch_list(&watch_list)?;
    Ok("✅ All price watches cleared".to_string())
}

fn reset_triggered() -> Result<String, Box<dyn Error>> {
    let mut watch_list = load_watch_list()?;
    watch_list.reset_all_states();
    save_watch_list(&watch_list)?;

    let triggered_count = watch_list
        .watches
        .iter()
        .filter(|w| !w.triggered)
        .count();

    Ok(format!(
        "✅ Reset triggered state for {} watches",
        triggered_count
    ))
}

fn get_help_text() -> String {
    r#"🚀 Ticker - Price Watch Management

Usage: ticker [COMMAND] [OPTIONS]

Commands:
  add <asset> <price> <above|below>
    Add a price watch
    Example: ticker add Bitcoin 68000 above

  remove <asset> <price>
    Remove a price watch
    Example: ticker remove Bitcoin 68000

  list
    List all active price watches
    Example: ticker list

  clear
    Clear all price watches
    Example: ticker clear

  reset
    Reset triggered state for all watches
    Example: ticker reset

  help, --help, -h
    Show this help message

Examples:
  # Add a watch for BTC above €68,000
  ticker add Bitcoin 68000 above

  # Add a watch for Gold below €2,000
  ticker add Gold 2000 below

  # List all watches
  ticker list

  # Remove a watch
  ticker remove Bitcoin 68000

Notes:
  - Watches are persisted in ~/.ticker_watches.json
  - When a price reaches the watch threshold, a notification will be sent
  - Each watch will only trigger once per day
  - Running without arguments starts the menubar app
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_text_is_not_empty() {
        let help = get_help_text();
        assert!(!help.is_empty());
        assert!(help.contains("add"));
        assert!(help.contains("remove"));
        assert!(help.contains("list"));
    }
}
