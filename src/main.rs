use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

mod config;
mod formatter;
mod menu_builder;
mod menubar;
mod poller;
mod price_fetcher;

fn log_file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".ticker_debug.log")
}

fn log_message(message: &str) {
    // ALTIJD naar stderr (zichtbaar in Terminal)
    eprintln!("{}", message);

    // PLUS naar logfile
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path())
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    }
}

fn main() {
    // Clear old log
    let log_path = log_file_path();
    let _ = std::fs::remove_file(&log_path);

    log_message("=== TICKER APP STARTED ===");
    log_message(&format!("Log file: {:?}", log_path));
    log_message(&format!("Working dir: {:?}", std::env::current_dir()));
    log_message(&format!("Executable: {:?}", std::env::current_exe()));

    match menubar::run_menubar() {
        Ok(_) => log_message("✓ Ticker app exited normally"),
        Err(e) => {
            let error_msg = format!("✗ FATAL ERROR: {}", e);
            log_message(&error_msg);
            std::process::exit(1);
        }
    }
}
