use image::ImageReader;
use polars::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};
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
use crate::price_fetcher::PriceFetcher;
use crate::price_history;

/// Fixed poll interval for all assets.
const POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);

struct App {
    tray: Rc<RefCell<TrayIcon>>,
    fetcher: Option<PriceFetcher>,
    config: Option<crate::config::Config>,
    /// Latest prices DataFrame (symbol, name, price, unit, prev_price, change, pct_change, direction).
    prices_df: Option<DataFrame>,
    links: HashMap<String, String>,
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
                    self.config = Some(config);
                    self.config_error = None;

                    if self.fetcher.is_some() {
                        eprintln!("💰 Fetching initial prices...");
                        self.poll_prices();
                        self.schedule_next_poll();
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
                        self.poll_prices();
                        self.schedule_next_poll();
                    } else {
                        eprintln!("⚠️  Cannot poll: config not loaded");
                    }
                }
                id => {
                    if let Some(url) = self.links.get(id) {
                        let _ = webbrowser::open(url);
                    }
                }
            }
        }

        if self.config.is_some() && SystemTime::now() >= self.next_check {
            eprintln!("⏰ Auto-poll triggered (every 5 minutes)");
            self.poll_prices();
            self.schedule_next_poll();
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
    fn poll_prices(&mut self) {
        let Some(config) = &self.config else { return };
        let Some(fetcher) = &self.fetcher else { return };

        let result = match &self.prices_df {
            None => {
                eprintln!("📊 First poll — constructing DataFrame");
                fetcher.build_initial_dataframe(&config.assets)
            }
            Some(previous) => {
                eprintln!("📊 Updating existing DataFrame");
                fetcher.update_dataframe(previous, &config.assets)
            }
        };

        let df = match result {
            Ok(df) => df,
            Err(e) => {
                eprintln!("✗ Poll failed: {}", e);
                return;
            }
        };

        // Persist current prices for next app launch (optional baseline)
        let mut history = HashMap::new();
        if let (Ok(names), Ok(prices)) = (df.column("name"), df.column("price"))
            && let (Ok(name_ca), Ok(price_ca)) = (names.str(), prices.f64())
        {
            for i in 0..df.height() {
                if let (Some(name), Some(price)) = (name_ca.get(i), price_ca.get(i))
                    && !price.is_nan()
                {
                    history.insert(name.to_string(), price);
                }
            }
        }
        if !history.is_empty()
            && let Err(e) = price_history::save_price_history(&history)
        {
            eprintln!("⚠️  Failed to save price history: {}", e);
        }

        self.prices_df = Some(df);
        self.update_menu();
    }

    fn schedule_next_poll(&mut self) {
        self.next_check = SystemTime::now() + POLL_INTERVAL;
        eprintln!("⏱️  Next poll in {}s", POLL_INTERVAL.as_secs());
    }

    fn has_changes(&self) -> bool {
        let Some(df) = &self.prices_df else {
            return false;
        };
        let Ok(directions) = df.column("direction") else {
            return false;
        };
        let Ok(dir_ca) = directions.str() else {
            return false;
        };

        for i in 0..df.height() {
            match dir_ca.get(i) {
                Some("up") | Some("down") => return true,
                _ => {}
            }
        }
        false
    }

    fn update_menu(&self) {
        let empty = DataFrame::new(vec![
            Series::new("symbol".into(), Vec::<String>::new()).into(),
            Series::new("name".into(), Vec::<String>::new()).into(),
            Series::new("price".into(), Vec::<f64>::new()).into(),
            Series::new("unit".into(), Vec::<String>::new()).into(),
            Series::new("prev_price".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("change".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("pct_change".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("direction".into(), Vec::<String>::new()).into(),
        ])
        .expect("empty dataframe");

        let df = self.prices_df.clone().unwrap_or(empty);
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

    let mut links = HashMap::new();
    links.insert("bitcoin".to_string(), "https://bitcoin.nl".to_string());
    links.insert("eth".to_string(), "https://bitcoin.nl".to_string());
    links.insert("gold".to_string(), "https://xaus.com".to_string());
    links.insert("gas".to_string(), "https://eurooilwatch.com".to_string());
    links.insert(
        "benzine".to_string(),
        "https://eurooilwatch.com".to_string(),
    );
    links.insert("diesel".to_string(), "https://eurooilwatch.com".to_string());

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
        config: None,
        prices_df: None,
        links,
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
