use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tray_icon::{TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent}};
use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}};
use webbrowser;

use crate::config::load_config;
use crate::price_fetcher::PriceFetcher;
use crate::poller::Poller;
use crate::menu_builder::MenuBuilder;

struct App {
    tray: Arc<Mutex<TrayIcon>>,
    fetcher: PriceFetcher,
    poller: Poller,
    config: crate::config::Config,
    links: HashMap<String, String>,
    prices: Vec<(String, f64, String)>,
    price_history: HashMap<String, f64>,
    next_check: SystemTime,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        _event: WindowEvent,
    ) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Check for menu clicks
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => event_loop.exit(),
                "poll" => {
                    eprintln!("🔄 Manual poll triggered");
                    self.poll_due_assets(true);
                    self.update_next_check();
                }
                id => {
                    if let Some(url) = self.links.get(id) {
                        let _ = webbrowser::open(url);
                    }
                }
            }
        }

        // Auto-poll when due
        if SystemTime::now() >= self.next_check {
            eprintln!("⏰ Auto-poll triggered");
            self.poll_due_assets(false);
            self.update_next_check();
        }

        // Update control flow to wake up at next check time
        if let Ok(duration) = self.next_check.duration_since(SystemTime::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + duration
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

impl App {
    fn poll_due_assets(&mut self, force: bool) {
        let new_prices = self.fetcher.fetch_all(&self.config.assets);

        eprintln!("🔍 Checking {} assets for updates...", new_prices.len());

        let mut updated_count = 0;
        let mut updates = Vec::new();

        for (new_name, new_price, new_unit) in &new_prices {
            // Skip if not due AND not forced
            if !force && !self.poller.should_poll(new_name) {
                eprintln!("  ⏭️  {} (not due yet)", new_name);
                continue;
            }

            // Get OLD price
            let old_price = self.price_history.get(new_name).copied();

            let price_str = if new_price.is_nan() { "?".to_string() } else { format!("{:.2}", new_price) };
            let change_note = if let Some(old) = old_price {
                if (old - new_price).abs() < 0.01 {
                    " (no change)".to_string()
                } else {
                    let diff = new_price - old;
                    let sign = if diff > 0.0 { "+" } else { "" };
                    format!(" ({}{:.2})", sign, diff)
                }
            } else {
                "".to_string()
            };

            // Update current prices
            if let Some(pos) = self.prices.iter().position(|(name, _, _)| name == new_name) {
                self.prices[pos] = (new_name.clone(), *new_price, new_unit.clone());
            } else {
                self.prices.push((new_name.clone(), *new_price, new_unit.clone()));
            }

            self.poller.mark_polled(new_name, &self.config.assets);
            eprintln!("  ✓ {} → {} {}{}", new_name, price_str, new_unit, change_note);
            updated_count += 1;

            updates.push((new_name.clone(), *new_price));
        }

        eprintln!("📊 Update complete: {} updated", updated_count);

        self.update_menu();

        for (name, price) in updates {
            self.price_history.insert(name, price);
        }
    }

    fn update_next_check(&mut self) {
        let mut earliest = SystemTime::now() + std::time::Duration::from_secs(3600);
        let mut earliest_asset = "unknown".to_string();

        for asset in &self.config.assets {
            if let Some(duration) = self.poller.time_until_poll(&asset.name) {
                let next = SystemTime::now() + duration;
                if next < earliest {
                    earliest = next;
                    earliest_asset = asset.name.clone();
                }
            }
        }

        self.next_check = earliest;
        let secs = self.next_check.duration_since(SystemTime::now()).unwrap_or_default().as_secs();
        eprintln!("⏱️  Next check: {}s ({})", secs, earliest_asset);
    }

    fn update_menu(&self) {
        let prices_with_history: Vec<(String, f64, String, Option<f64>)> = self.prices
            .iter()
            .map(|(name, price, unit)| {
                let prev = self.price_history.get(name).copied();
                (name.clone(), *price, unit.clone(), prev)
            })
            .collect();

        let menu = MenuBuilder::build(&prices_with_history, &self.poller);

        if let Ok(tray) = self.tray.lock() {
            tray.set_menu(Some(Box::new(menu)));
        }
    }
}

pub fn run_menubar() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🎯 Ticker app started");

    let config = load_config()?;
    let fetcher = PriceFetcher::new()?;
    let poller = Poller::new(&config.assets);

    let mut links = HashMap::new();
    links.insert("bitcoin".to_string(), "https://bitcoin.nl".to_string());
    links.insert("gold".to_string(), "https://xaus.com".to_string());
    links.insert("ttf_gas".to_string(), "https://eurooilwatch.com".to_string());

    let initial_menu = Menu::new();
    let _ = initial_menu.append(&MenuItem::new("⏳ Loading prices...", false, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("poll", "🔄  Poll now", true, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("quit", " Quit", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_tooltip("Price Ticker")
        .with_title("Ticker")
        .build()?;

    let tray = Arc::new(Mutex::new(tray_icon));

    let event_loop = EventLoop::new()?;

    let mut app = App {
        tray,
        fetcher,
        poller,
        config,
        links,
        prices: Vec::new(),
        price_history: HashMap::new(),
        next_check: SystemTime::now(),
    };

    app.poll_due_assets(true);
    app.update_next_check();

    event_loop.run_app(&mut app)?;
    Ok(())
}
