use image::ImageReader;
use polars::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
};

use crate::config::load_config;
use crate::menu_builder::MenuBuilder;
use crate::poller::Poller;
use crate::price_fetcher::PriceFetcher;
use crate::price_history;

struct App {
    tray: Rc<RefCell<TrayIcon>>,
    fetcher: Option<PriceFetcher>,
    poller: Option<Poller>,
    config: Option<crate::config::Config>,
    /// Latest fetch of all assets as one DataFrame (symbol, name, price, unit).
    prices_df: Option<DataFrame>,
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
        if !self.config_loaded {
            self.config_loaded = true;

            eprintln!("📂 Loading config...");

            match load_config() {
                Ok(config) => {
                    eprintln!("✓ Config loaded successfully");
                    self.config = Some(config.clone());
                    self.poller = Some(Poller::new(&config.assets));
                    self.config_error = None;

                    match price_history::load_price_history() {
                        Ok(history) => {
                            for (name, snapshot) in history {
                                self.price_history.insert(name, snapshot.value);
                            }
                            eprintln!("✓ Loaded price history from disk");
                        }
                        Err(_) => {
                            eprintln!("ℹ️  No previous price history found");
                        }
                    }

                    if self.fetcher.is_some() {
                        eprintln!("💰 Fetching initial prices...");
                        self.poll_due_assets(true);
                        self.update_menu();
                        self.update_next_check();
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to load config: {}", e);
                    self.config_error = Some(format!("Config error: {}", e));
                    self.update_error_menu();
                }
            }

            return;
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => event_loop.exit(),
                "poll" => {
                    eprintln!("🔄 Manual poll triggered");
                    if self.config.is_some() {
                        self.poll_due_assets(true);
                        self.update_next_check();
                    } else {
                        eprintln!("⚠️  Cannot poll: config not loaded");
                    }
                }
                _ => {}
            }
        }

        if self.config.is_some() && SystemTime::now() >= self.next_check {
            eprintln!("⏰ Auto-poll triggered");
            self.poll_due_assets(false);
            self.update_next_check();
        }

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

        if !force {
            let any_due = config.assets.iter().any(|a| poller.should_poll(&a.name));
            if !any_due {
                eprintln!("⏭️  No assets due yet");
                return;
            }
        }

        let df = match fetcher.fetch_all(&config.assets) {
            Ok(df) => df,
            Err(e) => {
                eprintln!("✗ Failed to build prices DataFrame: {}", e);
                return;
            }
        };

        eprintln!("🔍 Checking {} assets for updates...", df.height());

        let mut updated_count = 0;
        let mut pending_history: Vec<(String, f64)> = Vec::new();

        let name_col = df.column("name").ok();
        let price_col = df.column("price").ok();

        if let (Some(names), Some(prices)) = (name_col, price_col) {
            let name_ca = names.str().ok();
            let price_ca = prices.f64().ok();

            if let (Some(name_ca), Some(price_ca)) = (name_ca, price_ca) {
                for i in 0..df.height() {
                    let Some(name) = name_ca.get(i) else { continue };
                    let Some(new_price) = price_ca.get(i) else { continue };

                    let old_price = self.price_history.get(name).copied();
                    let price_changed = if let Some(old) = old_price {
                        (old - new_price).abs() > 0.01
                    } else {
                        true
                    };

                    poller.mark_polled(name, &config.assets);

                    if price_changed {
                        updated_count += 1;
                        pending_history.push((name.to_string(), new_price));
                    }

                    let price_str = if new_price.is_nan() {
                        "?".to_string()
                    } else {
                        format!("{:.2}", new_price)
                    };
                    eprintln!("  ✓ {} → {}", name, price_str);
                }
            }
        }

        eprintln!("📊 Update complete: {} updated", updated_count);

        self.prices_df = Some(df);

        // Menu/icon first while history still holds previous values
        if updated_count > 0 || force {
            self.update_menu();
        }

        for (name, price) in pending_history {
            self.price_history.insert(name, price);
        }

        if updated_count > 0 {
            if let Err(e) = price_history::save_price_history(&self.price_history) {
                eprintln!("⚠️  Failed to save price history: {}", e);
            } else {
                eprintln!("💾 Price history saved");
            }
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
        let Some(df) = &self.prices_df else {
            return false;
        };
        let Ok(names) = df.column("name") else {
            return false;
        };
        let Ok(prices) = df.column("price") else {
            return false;
        };
        let Ok(name_ca) = names.str() else {
            return false;
        };
        let Ok(price_ca) = prices.f64() else {
            return false;
        };

        for i in 0..df.height() {
            let Some(name) = name_ca.get(i) else { continue };
            let Some(price) = price_ca.get(i) else { continue };
            if let Some(prev) = self.price_history.get(name)
                && !price.is_nan()
                && !prev.is_nan()
                && (price - prev).abs() > 0.01
            {
                return true;
            }
        }
        false
    }

    fn update_menu(&self) {
        let df = self.prices_df.clone().unwrap_or_else(|| {
            DataFrame::new(vec![
                Series::new("symbol".into(), Vec::<String>::new()).into(),
                Series::new("name".into(), Vec::<String>::new()).into(),
                Series::new("price".into(), Vec::<f64>::new()).into(),
                Series::new("unit".into(), Vec::<String>::new()).into(),
            ])
            .expect("empty dataframe")
        });

        let menu = MenuBuilder::build(&df);

        if let Ok(tray) = self.tray.try_borrow_mut() {
            tray.set_menu(Some(Box::new(menu)));

            let icon = if self.has_changes() {
                self.alert_icon.clone()
            } else {
                self.normal_icon.clone()
            };

            let _ = tray.set_icon(Some(icon));
            tray.set_title(Some("Ticker"));
        }
    }

    fn update_error_menu(&self) {
        let menu = Menu::new();

        if let Some(error) = &self.config_error {
            let _ = menu.append(&MenuItem::new(format!("❌ {}", error), false, None));
        } else {
            let _ = menu.append(&MenuItem::new("⏳ Loading config...", false, None));
        }

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id("poll", "🔄 Retry", true, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuBuilder::version_item());
        let _ = menu.append(&MenuItem::with_id("quit", " Quit", true, None));

        if let Ok(tray) = self.tray.try_borrow_mut() {
            tray.set_menu(Some(Box::new(menu)));
        }
    }
}

fn bundle_assets_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(app_dir) = exe_path.ancestors().find(|p| {
            p.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with(".app"))
                .unwrap_or(false)
        })
    {
        let assets = app_dir.join("Contents/Resources/assets");
        if assets.exists() {
            return assets;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn load_icon(filename: &str) -> Result<Icon, Box<dyn std::error::Error>> {
    let assets_dir = bundle_assets_dir();
    let icon_path = assets_dir.join(filename);

    eprintln!("📁 Loading icon from: {:?}", icon_path);

    let image = ImageReader::open(&icon_path)?.decode()?.to_rgba8();
    let (width, height) = image.dimensions();
    let icon = Icon::from_rgba(image.into_raw(), width, height)?;
    Ok(icon)
}

pub fn run_menubar() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🎯 Ticker app started");

    let fetcher = PriceFetcher::new()?;

    eprintln!("📁 Loading icons...");
    let normal_icon = load_icon("normal.png")?;
    let alert_icon = load_icon("update.png")?;

    eprintln!("🎨 Creating initial menubar...");
    let initial_menu = Menu::new();
    let _ = initial_menu.append(&MenuItem::new("⏳ Loading config...", false, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuItem::with_id("poll", "🔄 Retry", true, None));
    let _ = initial_menu.append(&PredefinedMenuItem::separator());
    let _ = initial_menu.append(&MenuBuilder::version_item());
    let _ = initial_menu.append(&MenuItem::with_id("quit", " Quit", true, None));

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_icon(normal_icon.clone())
        .with_tooltip("Price Ticker")
        .with_title("Ticker")
        .build()?;

    eprintln!("✓ Menubar is visible");

    let tray = Rc::new(RefCell::new(tray_icon));
    let event_loop = EventLoop::new()?;

    let mut app = App {
        tray,
        fetcher: Some(fetcher),
        poller: None,
        config: None,
        prices_df: None,
        price_history: HashMap::new(),
        next_check: SystemTime::now(),
        normal_icon,
        alert_icon,
        config_loaded: false,
        config_error: None,
    };

    eprintln!("🚀 Starting event loop...");
    event_loop.run_app(&mut app)?;
    Ok(())
}
