use crate::config::load_config;
use crate::menu_builder::build_menu_with_next_poll;
use crate::models::Price;
use crate::price_fetcher::fetch_prices;
use crate::price_tracker;

use chrono::{Duration as ChronoDuration, Local};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use webbrowser;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

struct App {
    tray: Arc<Mutex<TrayIcon>>,
    links: HashMap<String, String>,
    startup_fetch_done: bool,
    fetch_in_progress: bool,
    last_poll_time: Instant,
    prices: Vec<Price>,
    next_poll_time_str: String,
    last_update_timestamp: String,
    tx: mpsc::Sender<Vec<Price>>,
    rx: mpsc::Receiver<Vec<Price>>,
}

fn current_timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn format_next_poll_time(seconds_until: u64) -> String {
    let next_time = Local::now() + ChronoDuration::seconds(seconds_until as i64);
    next_time.format("%H:%M:%S").to_string()
}

fn market_indicator_with_counts(prices: &[Price]) -> (String, u32, u32) {
    let mut up_count = 0u32;
    let mut down_count = 0u32;

    for price in prices {
        if let Some(change) = price_tracker::get_price_change(&price.name, price.value) {
            if change.contains("🟢") {
                up_count += 1;
            } else if change.contains("🔴") {
                down_count += 1;
            }
        }
    }

    let total = prices.len() as u32;

    let indicator = if up_count > down_count {
        format!("▲ {}/{}", up_count, total)
    } else if down_count > up_count {
        format!("▼ {}/{}", down_count, total)
    } else {
        format!("→ {}/{}", up_count, total)
    };

    (indicator, up_count, down_count)
}

fn spawn_price_fetcher(tx: mpsc::Sender<Vec<Price>>) {
    thread::spawn(move || {
        if let Ok(config) = load_config() {
            if let Ok(prices) = fetch_prices(&config.assets) {
                let _ = tx.send(prices);
            }
        }
    });
}

impl App {
    fn refresh_tray(&self) {
        let (indicator, _up, _down) = market_indicator_with_counts(&self.prices);
        let title = format!("Ticker {}", indicator);
        let menu = build_menu_with_next_poll(
            &self.prices,
            &self.next_poll_time_str,
            &self.last_update_timestamp,
        );

        if let Ok(tray) = self.tray.lock() {
            tray.set_title(Some(title.as_str()));
            tray.set_menu(Some(Box::new(menu)));
        }
    }

    fn start_background_fetch(&mut self) {
        if !self.fetch_in_progress {
            self.fetch_in_progress = true;
            spawn_price_fetcher(self.tx.clone());
        }
    }
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
        if !self.startup_fetch_done {
            self.startup_fetch_done = true;
            eprintln!("💰 Fetching initial prices...");

            if let Ok(config) = load_config() {
                match fetch_prices(&config.assets) {
                    Ok(prices) => {
                        eprintln!("✓ Got {} prices", prices.len());
                        self.prices = prices;
                        self.last_poll_time = Instant::now();
                        self.last_update_timestamp = current_timestamp();
                        self.next_poll_time_str = format_next_poll_time(30);
                        self.fetch_in_progress = false;
                        self.refresh_tray();
                    }
                    Err(e) => {
                        eprintln!("✗ Failed to fetch prices: {}", e);
                    }
                }
            }
        }

        if let Ok(new_prices) = self.rx.try_recv() {
            eprintln!("✓ Got prices from background thread");
            self.prices = new_prices;
            self.fetch_in_progress = false;
            self.last_poll_time = Instant::now();
            self.last_update_timestamp = current_timestamp();
            self.next_poll_time_str = format_next_poll_time(30);
            self.refresh_tray();
        }

        if self.last_poll_time.elapsed() >= Duration::from_secs(30) && !self.fetch_in_progress {
            eprintln!("⏱️ Auto-polling after 30 seconds");
            self.start_background_fetch();
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => {
                    event_loop.exit();
                }
                "poll" => {
                    eprintln!("🔄 Manual poll triggered");
                    self.start_background_fetch();
                }
                id if id.starts_with("pin_") => {
                    eprintln!("🔧 Pin event triggered: {}", id);

                    let price_name_raw = id.strip_prefix("pin_").unwrap_or("");

                    if let Some(price) = self
                        .prices
                        .iter()
                        .find(|p| p.name.to_lowercase().replace(' ', "_") == price_name_raw)
                    {
                        let price_name = price.name.clone();
                        let price_value = price.value;

                        if let Ok(pinned) = price_tracker::load_pinned_prices() {
                            if pinned.contains_key(&price_name) {
                                let _ = price_tracker::remove_pinned_price(&price_name);
                                eprintln!("📍 Unpinned: {}", price_name);
                            } else {
                                let _ = price_tracker::save_pinned_price(&price_name, price_value);
                                eprintln!("📌 Pinned: {} @ {}", price_name, price_value);
                            }
                        }

                        self.refresh_tray();
                    } else {
                        eprintln!("❌ Could not find price for: {}", price_name_raw);
                    }
                }
                id => {
                    if let Some(url) = self.links.get(id) {
                        let _ = webbrowser::open(url);
                    }
                }
            }
        }
    }
}

pub fn run_menubar() -> Result<(), Box<dyn Error>> {
    eprintln!("🎯 Ticker app started");
    eprintln!("🔧 Initializing menubar...");

    let mut links = HashMap::new();
    links.insert("bitcoin".to_string(), "https://bitcoin.nl".to_string());
    links.insert(
        "gold".to_string(),
        "https://www.inkoopedelmetaal.nl/goud-verkopen/gouden-munten".to_string(),
    );
    links.insert(
        "ttf_gas".to_string(),
        "https://tradingeconomics.com/commodity/eu-natural-gas".to_string(),
    );

    let initial_menu = Menu::new();
    let _ = initial_menu.append(&MenuItem::new("⏳ Loading prices...", false, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("poll", "🔄 Fetching...", true, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("quit", " Quit", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_tooltip("Price Ticker")
        .with_title("Ticker")
        .build()?;

    let tray = Arc::new(Mutex::new(tray_icon));

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let (tx, rx) = mpsc::channel();

    let now = Instant::now();
    let mut app = App {
        tray,
        links,
        startup_fetch_done: false,
        fetch_in_progress: false,
        last_poll_time: now,
        prices: Vec::new(),
        next_poll_time_str: "loading...".to_string(),
        last_update_timestamp: "never".to_string(),
        tx,
        rx,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
