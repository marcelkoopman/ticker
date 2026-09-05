mod config;
mod formatter;
mod menu_builder;
mod menubar;
mod models;
mod price_fetcher;
mod price_history;
mod singleton;

fn main() {
    eprintln!("🚀 Ticker app starting...");

    // Acquire singleton lock to prevent multiple instances
    if let Err(e) = singleton::try_acquire_lock() {
        eprintln!("✗ {}", e);
        std::process::exit(1);
    }

    // Run the menubar application
    match menubar::run_menubar() {
        Ok(_) => {
            eprintln!("✓ Ticker app exited normally");
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            singleton::release_lock();
            std::process::exit(1);
        }
    }

    // Release lock on clean exit
    singleton::release_lock();
}
