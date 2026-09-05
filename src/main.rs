mod config;
mod formatter;
mod menu_builder;
mod menubar;
mod models;
mod price_fetcher;
mod price_tracker;
mod singleton;

fn main() {
    eprintln!("🚀 Ticker app starting...");

    // Check singleton
    if let Err(e) = singleton::try_acquire_lock() {
        eprintln!("✗ {}", e);
        std::process::exit(1);
    }

    match menubar::run_menubar() {
        Ok(_) => eprintln!("✓ Ticker app exited normally"),
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }

    singleton::release_lock();
}
