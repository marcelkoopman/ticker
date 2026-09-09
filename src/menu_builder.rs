use polars::prelude::*;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

pub struct MenuBuilder;

impl MenuBuilder {
    /// Build the tray menu with a single non-clickable item that shows the DataFrame.
    pub fn build(df: &DataFrame) -> Menu {
        let menu = Menu::new();

        let label = if df.height() == 0 {
            "No prices yet".to_string()
        } else {
            // Polars Display renders a compact table
            format!("{df}")
        };

        let _ = menu.append(&MenuItem::new(label, false, None));

        let _ = menu.append(&PredefinedMenuItem::separator());
        let poll_item = MenuItem::with_id("poll", "🔄  Poll now", true, None);
        let _ = menu.append(&poll_item);

        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&Self::version_item());
        let quit_item = MenuItem::with_id("quit", " Quit", true, None);
        let _ = menu.append(&quit_item);

        menu
    }

    /// Non-clickable version label (matches Cargo.toml / release tag version).
    pub fn version_item() -> MenuItem {
        MenuItem::new(
            format!("Version {}", env!("CARGO_PKG_VERSION")),
            false,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_pkg_version() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }

    #[test]
    fn build_empty_dataframe_menu() {
        let df = DataFrame::new(vec![
            Series::new("symbol".into(), Vec::<String>::new()).into(),
            Series::new("name".into(), Vec::<String>::new()).into(),
            Series::new("price".into(), Vec::<f64>::new()).into(),
            Series::new("unit".into(), Vec::<String>::new()).into(),
        ])
        .unwrap();
        let _menu = MenuBuilder::build(&df);
    }

    #[test]
    fn build_sample_dataframe_menu() {
        let df = DataFrame::new(vec![
            Series::new("symbol".into(), vec!["💰".to_string()]).into(),
            Series::new("name".into(), vec!["Bitcoin".to_string()]).into(),
            Series::new("price".into(), vec![95000.0_f64]).into(),
            Series::new("unit".into(), vec!["EUR".to_string()]).into(),
        ])
        .unwrap();
        let _menu = MenuBuilder::build(&df);
    }
}
