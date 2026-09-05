use crate::config::load_config;
use crate::menu_builder::build_menu_with_next_poll;
use crate::models::Price;
use crate::price_fetcher::fetch_prices;
use crate::price_history;

use chrono::{Duration as ChronoDuration, Local};
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

struct AssetPoller {
    last_poll: Instant,
    #[allow(dead_code)]
    poll_interval_seconds: u64,
}

struct App {
    tray: Rc<RefCell<TrayIcon>>,
    links: HashMap<String, String>,
    startup_fetch_done: bool,
    fetch_in_progress: bool,
    prices: Vec<Price>,
    asset_pollers: HashMap<String, AssetPoller>,
    next_poll_time_str: String,
    last_update_timestamp: String,
    tx: mpsc::Sender<Vec<Price>>,
    rx: mpsc::Receiver<Vec<Price>>,
    asset_names: Vec<String>,
    poll_intervals: HashMap<String, u64>,
}

fn current_timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn format_next_poll_time(seconds_until: u64) -> String {
    let next_time = Local::now() + ChronoDuration::seconds(seconds_until as i64);
    next_time.format("%H:%M:%S").to_string()
}

fn spawn_price_fetcher(tx: mpsc::Sender<Vec<Price>>, _asset_names: Vec<String>) {
    thread::spawn(move || {
        if let Ok(config) = load_config()
            && let Ok(prices) = fetch_prices(&config.assets)
        {
            let _ = tx.send(prices);
        }
    });
}

impl App {
    fn refresh_tray(&self) {
        let title = "Ticker".to_string();
        let menu = build_menu_with_next_poll(
            &self.prices,
            &self.next_poll_time_str,
            &self.last_update_timestamp,
        );

        let tray = self.tray.borrow_mut();
        tray.set_title(Some(title.as_str()));
        tray.set_menu(Some(Box::new(menu)));
    }

    fn start_background_fetch(&mut self) {
        if !self.fetch_in_progress {
            self.fetch_in_progress = true;
            spawn_price_fetcher(self.tx.clone(), self.asset_names.clone());
        }
    }

    fn should_poll_any_asset(&self) -> bool {
        for asset_name in &self.asset_names {
            if let Some(poller) = self.asset_pollers.get(asset_name)
                && let Some(interval) = self.poll_intervals.get(asset_name)
                && poller.last_poll.elapsed() >= Duration::from_secs(*interval)
            {
                return true;
            }
            if !self.asset_pollers.contains_key(asset_name) {
                return true;
            }
        }
        false
    }

    fn update_asset_poll_times(&mut self) {
        for asset_name in &self.asset_names {
            if let Some(interval) = self.poll_intervals.get(asset_name) {
                self.asset_pollers.insert(
                    asset_name.clone(),
                    AssetPoller {
                        last_poll: Instant::now(),
                        poll_interval_seconds: *interval,
                    },
                );
            }
        }
    }

    fn calculate_next_poll_time(&self) -> String {
        let mut min_seconds = u64::MAX;

        for asset_name in &self.asset_names {
            if let Some(poller) = self.asset_pollers.get(asset_name)
                && let Some(interval) = self.poll_intervals.get(asset_name)
            {
                let elapsed = poller.last_poll.elapsed().as_secs();
                let remaining = interval.saturating_sub(elapsed);
                if remaining < min_seconds {
                    min_seconds = remaining;
                }
            }
        }

        if min_seconds == u64::MAX {
            return "calculating...".to_string();
        }
        format_next_poll_time(min_seconds)
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
                        self.prices = prices.clone();

                        for price in &prices {
                            let _ = price_history::update_price_history(&price.name, price.value);
                        }

                        self.update_asset_poll_times();
                        self.last_update_timestamp = current_timestamp();
                        self.next_poll_time_str = self.calculate_next_poll_time();
                        self.fetch_in_progress = false;
                        self.refresh_tray();
                    }
                    Err(e) => {
                        eprintln!("✗ Failed to fetch prices: {}", e);
                    }
                }
            }
            return;
        }

        if let Ok(new_prices) = self.rx.try_recv() {
            eprintln!("✓ Got prices from background thread");
            self.prices = new_prices.clone();

            for price in &self.prices {
                let _ = price_history::update_price_history(&price.name, price.value);
            }

            self.fetch_in_progress = false;
            self.update_asset_poll_times();
            self.last_update_timestamp = current_timestamp();
            self.next_poll_time_str = self.calculate_next_poll_time();
            self.refresh_tray();
        }

        if self.should_poll_any_asset() && !self.fetch_in_progress {
            eprintln!("⏱️ Auto-polling triggered");
            self.start_background_fetch();
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => {
                    eprintln!("👋 Quitting...");
                    event_loop.exit();
                }
                "poll" => {
                    eprintln!("🔄 Manual poll triggered");
                    self.start_background_fetch();
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

pub fn initialize_app() -> Result<(), Box<dyn Error>> {
    eprintln!("🚀 Ticker app starting...");
    eprintln!("🔧 Initializing menubar...");

    let config = load_config()?;

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

    let mut asset_names = Vec::new();
    let mut poll_intervals = HashMap::new();

    for asset in &config.assets {
        asset_names.push(asset.name.clone());
        poll_intervals.insert(asset.name.clone(), asset.poll_interval_seconds());
    }

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

    let tray = Rc::new(RefCell::new(tray_icon));

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let (tx, rx) = mpsc::channel();

    let mut app = App {
        tray,
        links,
        startup_fetch_done: false,
        fetch_in_progress: false,
        prices: Vec::new(),
        asset_pollers: HashMap::new(),
        next_poll_time_str: "loading...".to_string(),
        last_update_timestamp: "never".to_string(),
        tx,
        rx,
        asset_names,
        poll_intervals,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

pub fn run_menubar() -> Result<(), Box<dyn Error>> {
    initialize_app()
}
