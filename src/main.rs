mod menubar;

fn main() {
    eprintln!("🚀 Ticker app starting...");

    match menubar::run_menubar() {
        Ok(_) => {
            eprintln!("✓ Ticker app exited normally");
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            std::process::exit(1);
        }
    }
}
