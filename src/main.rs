mod menubar;
mod config;
mod price_fetcher;
mod formatter;
mod menu_builder;
mod models;
mod price_tracker;

fn main() {
    eprintln!("🚀 Ticker app starting...");

    match menubar::run_menubar() {
        Ok(_) => eprintln!("✓ Ticker app exited normally"),
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }
}
