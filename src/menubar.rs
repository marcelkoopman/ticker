use image::ImageReader;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use webbrowser;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

use crate::config::load_config;
use crate::menu_builder::MenuBuilder;
use crate::poller::Poller;
use crate::price_fetcher::PriceFetcher;

struct App {
    tray: Arc<Mutex<TrayIcon>>,
    fetcher: PriceFetcher,
    poller: Poller,
    config: crate::config::Config,
    links: HashMap<String, String>,
    prices: Vec<(String, f64, String)>,  // name, price, unit
    price_history: HashMap<String, f64>, // previous price per asset
    next_check: SystemTime,
    normal_icon: Icon,
    alert_icon: Icon,
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
        // Handle menu events
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => event_loop.exit(),
                "poll" => {
                    eprintln!("🔄 Manual poll triggered");
                    self.poll_due_assets(true); // force all assets
                    self.update_next_check();
                }
                id => {
                    if let Some(url) = self.links.get(id) {
                        let _ = webbrowser::open(url);
                    }
                }
            }
        }

        // Auto‑poll when due
        if SystemTime::now() >= self.next_check {
            eprintln!("⏰ Auto-poll triggered");
            self.poll_due_assets(false); // only due assets
            self.update_next_check();
        }

        // Sleep until next_check
        if let Ok(duration) = self.next_check.duration_since(SystemTime::now()) {
            event_loop
                .set_control_flow(ControlFlow::WaitUntil(std::time::Instant::now() + duration));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

impl App {
    /// Poll assets, optionally forcing all to update (`force = true` for "Poll now")
    fn poll_due_assets(&mut self, force: bool) {
        let new_prices = self.fetcher.fetch_all(&self.config.assets);

        eprintln!("🔍 Checking {} assets for updates...", new_prices.len());

        let mut updated_count = 0;
        let mut updates: Vec<(String, f64)> = Vec::new();

        for (new_name, new_price, new_unit) in &new_prices {
            // Skip if not due AND not forced
            if !force && !self.poller.should_poll(new_name) {
                eprintln!("  ⏭️  {} (not due yet)", new_name);
                continue;
            }

            // OLD price (from previous poll)
            let old_price = self.price_history.get(new_name).copied();

            let price_str = if new_price.is_nan() {
                "?".to_string()
            } else {
                format!("{:.2}", new_price)
            };

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

            // Update current prices list
            if let Some(pos) = self.prices.iter().position(|(name, _, _)| name == new_name) {
                self.prices[pos] = (new_name.clone(), *new_price, new_unit.clone());
            } else {
                self.prices
                    .push((new_name.clone(), *new_price, new_unit.clone()));
            }

            // Update poll schedule for this asset
            self.poller.mark_polled(new_name, &self.config.assets);
            eprintln!(
                "  ✓ {} → {} {}{}",
                new_name, price_str, new_unit, change_note
            );
            updated_count += 1;

            // Store update for history (applied AFTER menu update)
            updates.push((new_name.clone(), *new_price));
        }

        eprintln!("📊 Update complete: {} updated", updated_count);

        // Build menu using OLD prices from history
        self.update_menu();

        // Now update history for next comparison
        for (name, price) in updates {
            self.price_history.insert(name, price);
        }
    }

    /// Decide when to check next (smallest time_until_poll over all assets)
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
        let secs = self
            .next_check
            .duration_since(SystemTime::now())
            .unwrap_or_default()
            .as_secs();
        eprintln!("⏱️  Next check: {}s ({})", secs, earliest_asset);
    }

    /// Check if any asset has changed price (for alert icon)
    fn has_changes(&self) -> bool {
        for (name, price, _unit) in &self.prices {
            if let Some(prev) = self.price_history.get(name) {
                if !price.is_nan() && !prev.is_nan() {
                    let diff = price - prev;
                    if diff.abs() > 0.01 {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Build menu and update tray icon/title
    fn update_menu(&self) {
        // Enrich prices with previous price and symbol from config
        let prices_with_history: Vec<(String, f64, String, Option<f64>, String)> = self
            .prices
            .iter()
            .map(|(name, price, unit)| {
                let prev = self.price_history.get(name).copied();
                let symbol = self
                    .config
                    .assets
                    .iter()
                    .find(|a| a.name == *name)
                    .map(|a| a.symbol.clone())
                    .unwrap_or_else(|| "•".to_string());

                (name.clone(), *price, unit.clone(), prev, symbol)
            })
            .collect();

        let menu = MenuBuilder::build(&prices_with_history, &self.poller);

        if let Ok(tray) = self.tray.lock() {
            tray.set_menu(Some(Box::new(menu)));

            // Icon: alert if changes detected, otherwise normal
            let icon = if self.has_changes() {
                self.alert_icon.clone()
            } else {
                self.normal_icon.clone()
            };
            let _ = tray.set_icon(Some(icon));

            // Optional: update title (you can keep "Ticker" if you prefer)
            tray.set_title(Some("Ticker"));
        }
    }
}

/// Load a PNG icon and convert to `tray_icon::Icon`
fn load_icon(path: &str) -> Result<Icon, Box<dyn std::error::Error>> {
    let image = ImageReader::open(path)?.decode()?.to_rgba8();
    let (width, height) = image.dimensions();
    let icon = Icon::from_rgba(image.into_raw(), width, height)?;
    Ok(icon)
}

pub fn run_menubar() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🎯 Ticker app started");

    let config = load_config()?;
    let fetcher = PriceFetcher::new()?;
    let poller = Poller::new(&config.assets);

    // Load icons (32×32 PNG recommended)
    eprintln!("📁 Loading icons...");
    let normal_icon = load_icon("assets/normal.png")?;
    let alert_icon = load_icon("assets/update.png")?;

    // Links for click‑through on menu items
    let mut links = HashMap::new();
    links.insert("bitcoin".to_string(), "https://bitcoin.nl".to_string());
    links.insert("gold".to_string(), "https://xaus.com".to_string());
    links.insert(
        "ttf_gas".to_string(),
        "https://eurooilwatch.com".to_string(),
    );

    // Initial menu: loading state
    let initial_menu = Menu::new();
    let _ = initial_menu.append(&MenuItem::new("⏳ Loading prices...", false, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("poll", "🔄  Poll now", true, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("quit", " Quit", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_icon(normal_icon.clone())
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
        normal_icon,
        alert_icon,
    };

    // Initial forced poll (all assets)
    app.poll_due_assets(true);
    app.update_next_check();

    event_loop.run_app(&mut app)?;
    Ok(())
}
