use chrono::Local;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use webbrowser;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

#[derive(Debug, Deserialize)]
struct Config {
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    url: String,
    price_path: String,
    unit: String,
}

fn get_value_by_path(value: &Value, path: &str) -> Option<Value> {
    let mut current = value;
    for part in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(part)?,
            Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                arr.get(index)?
            }
            _ => return None,
        };
    }
    Some(current.clone())
}

fn fetch_prices() -> Result<Vec<(String, f64, String)>, Box<dyn Error>> {
    eprintln!("📋 Looking for config.toml in bundle...");

    let exe_path = std::env::current_exe()?;
    let config_path = exe_path
        .parent()
        .ok_or("Cannot determine executable parent")?
        .parent()
        .ok_or("Cannot determine Contents directory")?
        .join("Resources/config.toml");

    eprintln!("📂 Reading config from: {:?}", config_path);

    let config_str = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {:?}: {}", config_path, e))?;

    let config: Config = toml::from_str(&config_str)?;

    let client = Client::builder()
        .user_agent("rust-price-fetcher/1.0")
        .build()?;

    let mut results = Vec::new();

    for asset in config.assets {
        let price = match client.get(&asset.url).send() {
            Ok(response) => match response.error_for_status() {
                Ok(resp) => match resp.json::<Value>() {
                    Ok(json) => {
                        if let Some(price_value) = get_value_by_path(&json, &asset.price_path) {
                            match price_value {
                                Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
                                Value::String(s) => s.parse().unwrap_or(f64::NAN),
                                _ => f64::NAN,
                            }
                        } else {
                            f64::NAN
                        }
                    }
                    Err(_) => f64::NAN,
                },
                Err(_) => f64::NAN,
            },
            Err(_) => f64::NAN,
        };

        results.push((asset.name, price, asset.unit));
    }

    Ok(results)
}

fn current_timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
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

        // Add thousands separator to integer part
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

fn build_menu(prices: &[(String, f64, String)], timestamp: &str) -> Menu {
    let menu = Menu::new();

    // Prices with emoji symbols
    for (name, price, unit) in prices {
        let symbol = match name.as_str() {
            "Bitcoin" => "₿",
            "Gold" => "🟡",
            "TTF Gas" => "🔥",
            _ => "•",
        };

        let formatted_price = format_price(*price);

        // Format: Symbol Name — Price Unit
        let row = format!("{} {} — {} {}", symbol, name, formatted_price, unit);

        let item_id = name.to_lowercase().replace(" ", "_");
        let item = MenuItem::with_id(&item_id, &row, true, None);
        let _ = menu.append(&item);
    }

    let _ = menu.append(&PredefinedMenuItem::separator());

    // Combined "Poll again" + timestamp
    let poll_label = format!("🔄  Poll again ({})", timestamp);
    let poll_item = MenuItem::with_id("poll", &poll_label, true, None);
    let _ = menu.append(&poll_item);

    let _ = menu.append(&PredefinedMenuItem::separator());

    // Quit
    let quit_item = MenuItem::with_id("quit", " Quit", true, None);
    let _ = menu.append(&quit_item);

    menu
}

struct App {
    tray: Arc<Mutex<TrayIcon>>,
    links: HashMap<String, String>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        _event: WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => {
                    event_loop.exit();
                }
                "poll" => {
                    // Re-fetch prices and update the menu
                    if let Ok(prices) = fetch_prices() {
                        let timestamp = current_timestamp();
                        let new_menu = build_menu(&prices, &timestamp); // Remove &self.links

                        if let Ok(tray) = self.tray.lock() {
                            let _ = tray.set_menu(Some(Box::new(new_menu)));
                        }
                    }
                }
                id => {
                    // Handle asset links
                    if let Some(url) = self.links.get(id) {
                        let _ = webbrowser::open(url);
                    }
                }
            }
        }
    }
}

pub fn run_menubar() -> Result<(), Box<dyn Error>> {
    eprintln!("🔧 Initializing menubar...");

    let mut links = HashMap::new();
    links.insert("bitcoin".to_string(), "https://bitcoin.nl".to_string());
    links.insert("gold".to_string(), "https://www.inkoopedelmetaal.nl/goud-verkopen/gouden-munten".to_string());
    links.insert("ttf_gas".to_string(), "https://tradingeconomics.com/commodity/eu-natural-gas".to_string());

    eprintln!("💰 Fetching prices...");
    let prices = fetch_prices()?;
    eprintln!("✓ Got {} prices", prices.len());

    let timestamp = current_timestamp();
    eprintln!("⏰ Building menu with timestamp: {}", timestamp);
    let menu = build_menu(&prices, &timestamp);

    eprintln!("🎯 Creating tray icon...");
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Price Ticker")
        .with_title("ticker")
        .build()?;

    eprintln!("✓ Tray icon created, running event loop...");
    let tray = Arc::new(Mutex::new(tray_icon));
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App { tray, links };
    event_loop.run_app(&mut app)?;

    Ok(())
}
