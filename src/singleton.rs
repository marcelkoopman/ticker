use std::fs;
use std::path::PathBuf;
use std::error::Error;
use std::process;

fn get_lock_file_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".ticker.lock")
}

fn is_process_running(pid: u32) -> bool {
    // Check of process nog bestaat
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "macos"))]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

pub fn try_acquire_lock() -> Result<(), Box<dyn Error>> {
    let lock_file = get_lock_file_path();
    let current_pid = process::id();

    // Check of lock file bestaat
    if lock_file.exists() {
        // Lees PID uit lock file
        if let Ok(contents) = fs::read_to_string(&lock_file) {
            if let Ok(old_pid) = contents.trim().parse::<u32>() {
                // Check of oud proces nog draait
                if is_process_running(old_pid) {
                    eprintln!("❌ Ticker is already running (PID: {})!", old_pid);
                    return Err("Ticker is already running!".into());
                } else {
                    eprintln!("🧹 Cleaning up stale lock file (PID: {} not running)", old_pid);
                    let _ = fs::remove_file(&lock_file);
                }
            }
        }
    }

    // Maak nieuwe lock file met huidige PID
    fs::write(&lock_file, current_pid.to_string())?;
    eprintln!("🔒 Lock acquired (PID: {})", current_pid);
    Ok(())
}

pub fn release_lock() {
    let lock_file = get_lock_file_path();

    // Controleer of het onze lock is voordat we hem verwijderen
    if let Ok(contents) = fs::read_to_string(&lock_file) {
        if let Ok(pid) = contents.trim().parse::<u32>() {
            if pid == process::id() {
                let _ = fs::remove_file(&lock_file);
                eprintln!("🔓 Lock released");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_lock_success() {
        // Cleanup first
        let lock_file = get_lock_file_path();
        let _ = fs::remove_file(&lock_file);

        assert!(try_acquire_lock().is_ok());
        release_lock();
    }

    #[test]
    fn test_lock_contains_pid() {
        let lock_file = get_lock_file_path();
        let _ = fs::remove_file(&lock_file);

        let _ = try_acquire_lock();

        let contents = fs::read_to_string(&lock_file).unwrap();
        let pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(pid, process::id());

        release_lock();
    }

    #[test]
    fn test_stale_lock_cleanup() {
        let lock_file = get_lock_file_path();
        let _ = fs::remove_file(&lock_file);

        // Schrijf fake PID (9999 bestaat niet)
        fs::write(&lock_file, "9999").unwrap();

        // Acquire zou moeten slagen want 9999 draait niet
        assert!(try_acquire_lock().is_ok());
        release_lock();
    }
}
