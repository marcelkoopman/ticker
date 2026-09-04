use chrono::Local;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
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
    let config_str = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_str)?;

    let client = Client::builder()
        .user_agent("rust-price-fetcher/1.0")
        .build()?;

    let mut results = Vec::new();

    for asset in config.assets {
        let response = client
            .get(&asset.url)
            .send()?
            .error_for_status()?
            .json::<Value>()?;

        if let Some(price_value) = get_value_by_path(&response, &asset.price_path) {
            let price = match price_value {
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                Value::String(s) => s.parse().unwrap_or(0.0),
                _ => 0.0,
            };
            results.push((asset.name, price, asset.unit));
        }
    }

    Ok(results)
}

fn current_timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn build_menu(prices: &[(String, f64, String)], timestamp: &str) -> Menu {
    let menu = Menu::new();

    // Prices with symbols
    for (name, price, unit) in prices {
        let (symbol, label) = match name.as_str() {
            "Bitcoin" => ("₿", format!("Bitcoin   {:>10.2} {}", price, unit)),
            "Gold"    => ("🟡", format!("Gold      {:>10.2} {}", price, unit)),
            "TTF Gas" => ("🔥", format!("TTF Gas   {:>10.2} {}", price, unit)),
            _         => ("•",  format!("{}   {:>10.2} {}", name, price, unit)),
        };

        // true = normal (enabled) appearance
        let item = MenuItem::new(&format!("{}  {}", symbol, label), true, None);
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
                        let new_menu = build_menu(&prices, &timestamp);

                        if let Ok(tray) = self.tray.lock() {
                            let _ = tray.set_menu(Some(Box::new(new_menu)));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn run_menubar() -> Result<(), Box<dyn Error>> {
    // Initial fetch
    let prices = fetch_prices()?;
    let timestamp = current_timestamp();
    let menu = build_menu(&prices, &timestamp);

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Price Ticker")
        .with_title("ticker")
        .build()?;

    let tray = Arc::new(Mutex::new(tray_icon));

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App { tray };
    event_loop.run_app(&mut app)?;

    Ok(())
}
