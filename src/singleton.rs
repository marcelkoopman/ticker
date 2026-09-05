use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn get_lock_file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".ticker.lock")
}

pub fn try_acquire_lock() -> Result<(), Box<dyn Error>> {
    let lock_file = get_lock_file_path();

    if lock_file.exists() {
        eprintln!("❌ Ticker is already running!");
        return Err("Ticker is already running!".into());
    }

    fs::write(&lock_file, "")?;
    eprintln!("🔒 Lock acquired");
    Ok(())
}

pub fn release_lock() {
    let lock_file = get_lock_file_path();
    let _ = fs::remove_file(lock_file);
    eprintln!("🔓 Lock released");
}
