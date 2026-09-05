use image::ImageReader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
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
    tray: Rc<RefCell<TrayIcon>>,
    fetcher: Option<PriceFetcher>,
    poller: Option<Poller>,
    config: Option<crate::config::Config>,
    links: HashMap<String, String>,
    prices: Vec<(String, f64, String)>,
    price_history: HashMap<String, f64>,
    next_check: SystemTime,
    normal_icon: Icon,
    alert_icon: Icon,
    config_loaded: bool,
    config_error: Option<String>,
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
        // Load config once on first run
        if !self.config_loaded {
            eprintln!("📂 Loading config...");
            match load_config() {
                Ok(config) => {
                    eprintln!("✓ Config loaded successfully");
                    self.config = Some(config.clone());
                    self.poller = Some(Poller::new(&config.assets));
                    self.config_error = None;

                    // Fetch prices immediately after config loads
                    if let Some(_fetcher) = &self.fetcher {
                        eprintln!("💰 Fetching initial prices...");
                        self.poll_due_assets(true);
                        self.update_next_check();
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to load config: {}", e);
                    self.config_error = Some(format!("Config error: {}", e));
                    self.update_error_menu();
                }
            }
            self.config_loaded = true;
        }

        // Handle menu events
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => event_loop.exit(),
                "poll" => {
                    eprintln!("🔄 Manual poll triggered");
                    if self.config.is_some() {
                        self.poll_due_assets(true);
                        self.update_next_check();
                    }
                }
                id => {
                    if let Some(url) = self.links.get(id) {
                        let _ = webbrowser::open(url);
                    }
                }
            }
        }

        // Auto-poll when due
        if self.config.is_some() && SystemTime::now() >= self.next_check {
            eprintln!("⏰ Auto-poll triggered");
            self.poll_due_assets(false);
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
    fn poll_due_assets(&mut self, force: bool) {
        let Some(config) = &self.config else { return };
        let Some(fetcher) = &self.fetcher else { return };
        let Some(poller) = &mut self.poller else {
            return;
        };

        let new_prices = fetcher.fetch_all(&config.assets);
        eprintln!("🔍 Checking {} assets for updates...", new_prices.len());

        let mut updated_count = 0;
        let mut updates: Vec<(String, f64)> = Vec::new();

        for (new_name, new_price, new_unit) in &new_prices {
            if !force && !poller.should_poll(new_name) {
                eprintln!("  ⏭️  {} (not due yet)", new_name);
                continue;
            }

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

            if let Some(pos) = self.prices.iter().position(|(name, _, _)| name == new_name) {
                self.prices[pos] = (new_name.clone(), *new_price, new_unit.clone());
            } else {
                self.prices
                    .push((new_name.clone(), *new_price, new_unit.clone()));
            }

            poller.mark_polled(new_name, &config.assets);
            eprintln!(
                "  ✓ {} → {} {}{}",
                new_name, price_str, new_unit, change_note
            );
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
        let Some(config) = &self.config else { return };
        let Some(poller) = &self.poller else { return };

        let mut earliest = SystemTime::now() + std::time::Duration::from_secs(3600);
        let mut earliest_asset = "unknown".to_string();

        for asset in &config.assets {
            if let Some(duration) = poller.time_until_poll(&asset.name) {
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

    fn has_changes(&self) -> bool {
        for (name, price, _unit) in &self.prices {
            if let Some(prev) = self.price_history.get(name)
                && !price.is_nan()
                && !prev.is_nan()
            {
                let diff = price - prev;
                if diff.abs() > 0.01 {
                    return true;
                }
            }
        }
        false
    }

    fn update_menu(&self) {
        let Some(config) = &self.config else { return };
        let Some(poller) = &self.poller else { return };

        let prices_with_history: Vec<(String, f64, String, Option<f64>, String)> = self
            .prices
            .iter()
            .map(|(name, price, unit)| {
                let prev = self.price_history.get(name).copied();
                let symbol = config
                    .assets
                    .iter()
                    .find(|a| a.name == *name)
                    .map(|a| a.symbol.clone())
                    .unwrap_or_else(|| "•".to_string());

                (name.clone(), *price, unit.clone(), prev, symbol)
            })
            .collect();

        let menu = MenuBuilder::build(&prices_with_history, poller);

        match self.tray.try_borrow_mut() {
            Ok(tray) => {
                tray.set_menu(Some(Box::new(menu)));

                let icon = if self.has_changes() {
                    self.alert_icon.clone()
                } else {
                    self.normal_icon.clone()
                };
                let _ = tray.set_icon(Some(icon));
                tray.set_title(Some("Ticker"));
            }
            Err(_) => {}
        }
    }

    fn update_error_menu(&self) {
        let menu = Menu::new();

        if let Some(error) = &self.config_error {
            let _ = menu.append(&MenuItem::new(&format!("❌ {}", error), false, None));
        } else {
            let _ = menu.append(&MenuItem::new("⏳ Loading config...", false, None));
        }

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id("poll", "🔄  Retry", true, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id("quit", " Quit", true, None));

        match self.tray.try_borrow_mut() {
            Ok(tray) => {
                tray.set_menu(Some(Box::new(menu)));
            }
            Err(_) => {}
        }
    }
}

fn load_icon(path: &str) -> Result<Icon, Box<dyn std::error::Error>> {
    let image = ImageReader::open(path)?.decode()?.to_rgba8();
    let (width, height) = image.dimensions();
    let icon = Icon::from_rgba(image.into_raw(), width, height)?;
    Ok(icon)
}

pub fn run_menubar() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🎯 Ticker app started");

    let fetcher = PriceFetcher::new()?;

    eprintln!("📁 Loading icons...");
    let normal_icon = load_icon("assets/normal.png")?;
    let alert_icon = load_icon("assets/update.png")?;

    let mut links = HashMap::new();
    links.insert("bitcoin".to_string(), "https://bitcoin.nl".to_string());
    links.insert("gold".to_string(), "https://xaus.com".to_string());
    links.insert(
        "ttf_gas".to_string(),
        "https://eurooilwatch.com".to_string(),
    );

    // Initial menu: loading state
    let initial_menu = Menu::new();
    let _ = initial_menu.append(&MenuItem::new("⏳ Loading config...", false, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("poll", "🔄  Retry", true, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("quit", " Quit", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_icon(normal_icon.clone())
        .with_tooltip("Price Ticker")
        .with_title("Ticker")
        .build()?;

    let tray = Rc::new(RefCell::new(tray_icon));

    let event_loop = EventLoop::new()?;

    let mut app = App {
        tray,
        fetcher: Some(fetcher),
        poller: None,
        config: None,
        links,
        prices: Vec::new(),
        price_history: HashMap::new(),
        next_check: SystemTime::now(),
        normal_icon,
        alert_icon,
        config_loaded: false,
        config_error: None,
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}
