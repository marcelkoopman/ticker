use std::cell::RefCell;
use std::collections::HashMap;

use image::ImageFormat;
use polars::prelude::*;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::event_loop::{ControlFlow, EventLoopBuilder};

use crate::menu_builder::MenuBuilder;
use crate::price_watch::WatchList;

pub struct MenuBarApp {
    tray: RefCell<Option<TrayIcon>>,
    prices_df: Option<DataFrame>,
    normal_icon: Icon,
    alert_icon: Icon,
    watch_list: WatchList,
}

impl MenuBarApp {
    pub fn new() -> Self {
        let normal_icon = Self::create_fallback_icon(255, 255, 255); // Wit icoon
        let alert_icon = Self::create_fallback_icon(255, 0, 0);     // Rood icoon

        let watch_list = WatchList::new();

        let tray = TrayIconBuilder::new()
            .with_tooltip("Crypto & Asset Tracker")
            .with_icon(normal_icon.clone())
            .build()
            .ok();

        let app = Self {
            tray: RefCell::new(tray),
            prices_df: None,
            normal_icon,
            alert_icon,
            watch_list,
        };

        app.update_menu();
        app
    }

    pub fn update_prices(&mut self, df: DataFrame) {
        if let (Ok(names), Ok(prices)) = (df.column("name"), df.column("price")) {
            if let (Ok(names_str), Ok(prices_f64)) = (names.str(), prices.f64()) {
                for i in 0..df.height() {
                    if let (Some(name), Some(price)) = (names_str.get(i), prices_f64.get(i)) {
                        self.watch_list.check_price(name, price);
                    }
                }
            }
        }

        self.prices_df = Some(df);
        self.update_menu();
    }

    fn update_menu(&self) {
        let empty = DataFrame::new_infer_height(vec![
            Series::new("symbol".into(), Vec::<String>::new()).into(),
            Series::new("name".into(), Vec::<String>::new()).into(),
            Series::new("price".into(), Vec::<f64>::new()).into(),
            Series::new("unit".into(), Vec::<String>::new()).into(),
            Series::new("unit_hint".into(), Vec::<String>::new()).into(),
            Series::new("prev_price".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("change".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("pct_change".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("direction".into(), Vec::<String>::new()).into(),
            Series::new("day_open".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("change_day".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("pct_day".into(), Vec::<Option<f64>>::new()).into(),
            Series::new("direction_day".into(), Vec::<String>::new()).into(),
        ])
        .expect("empty dataframe");

        let df = self.prices_df.clone().unwrap_or(empty);
        let menu = MenuBuilder::build(&df, &self.watch_list);

        if let Ok(mut tray_opt) = self.tray.try_borrow_mut() {
            if let Some(tray) = tray_opt.as_mut() {
                let _ = tray.set_menu(Some(Box::new(menu)));

                let icon = if self.has_changes() {
                    self.alert_icon.clone()
                } else {
                    self.normal_icon.clone()
                };

                let _ = tray.set_icon(Some(icon));
                let _ = tray.set_title(Some("Ticker"));
            }
        }
    }

    fn has_changes(&self) -> bool {
        self.watch_list.watches.iter().any(|w| w.triggered)
    }

    fn create_fallback_icon(r: u8, g: u8, b: u8) -> Icon {
        let width = 16;
        let height = 16;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);

        for _ in 0..(width * height) {
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(255); // Alpha
        }

        Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
    }

    pub fn load_icon_from_memory(bytes: &[u8]) -> Icon {
        let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .expect("Failed to open icon path")
            .into_rgba8();

        let (width, height) = image.dimensions();
        let rgba = image.into_raw();

        Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
    }
}

pub fn run_menubar() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::new().build()?;
    let _app = MenuBarApp::new();

    event_loop.run(move |_event, target| {
        target.set_control_flow(ControlFlow::Wait);
    })?;

    Ok(())
}
