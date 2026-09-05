use crate::config::load_config;
use crate::price_fetcher::fetch_prices;
use crate::formatter::current_timestamp;
use crate::menu_builder::build_menu;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
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
                        let timestamp = current_timestamp();
                        let new_menu = build_menu(&prices, &timestamp);

                        if let Ok(tray) = self.tray.lock() {
                            tray.set_menu(Some(Box::new(new_menu)));
                        }
                    }
                    Err(e) => eprintln!("✗ Failed to fetch prices: {}", e),
                }
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "quit" => event_loop.exit(),
                "poll" => {
                    if let Ok(config) = load_config() {
                        if let Ok(prices) = fetch_prices(&config.assets) {
                            let timestamp = current_timestamp();
                            let new_menu = build_menu(&prices, &timestamp);

                            if let Ok(tray) = self.tray.lock() {
                                tray.set_menu(Some(Box::new(new_menu)));
                            }
                        }
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
    let _ = initial_menu.append(&MenuItem::with_id("poll", "🔄  Poll again", true, None));
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

    let mut app = App {
        tray,
        links,
        startup_fetch_done: false,
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}
