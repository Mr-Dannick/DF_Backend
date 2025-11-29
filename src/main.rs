use dotenv::dotenv;
mod kill_watcher;
mod database_setup;
mod player_monitor;

use std::env;
use std::path::Path;
use std::thread;
use crate::database_setup::setup_database;
use player_monitor::PlayerMonitor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let server_path = env::var("SERVER_PATH")?;
    let timeout: u64 = env::var("PLAYER_KILL_CHECKER_TIMEOUT")?.parse()?;
    let file_path = Path::new(&server_path).join("console.log");
    let database_setup = env::var("DATABASE_SETUP_COMPLETE")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true";

    // Only warn if an env var is actually missing; if `.env` contains these, no warnings will be printed.
    let database_user = env::var("DATABASE_USER")?;
    let database_password = env::var("DATABASE_PASSWORD")?;
    let database_ip = env::var("DATABASE_IP")?;
    let database_port = env::var("DATABASE_PORT")?;
    let database_name = env::var("DATABASE_NAME")?;

    println!("Database setup complete: {}", database_setup);
    if !database_setup {
        println!("Warning: Database setup is not complete. Some features may not work as expected.");
        setup_database(&database_ip, &database_port, &database_name, &database_user, &database_password);
    }
    check_env(file_path.to_str().unwrap())?;

    // Run player monitor and kill watcher simultaneously in separate threads.
    let monitor_path = file_path.to_str().unwrap().to_string();
    let monitor = PlayerMonitor::new(&monitor_path);

    let monitor_handle = thread::spawn(move || {
        monitor.start_monitoring();
    });

    let watcher_handle = thread::spawn(move || {
        kill_watcher::watch_console_log(file_path, timeout);
    });

    // Wait for both (these are long-running loops; joining will block here).
    let _ = monitor_handle.join();
    let _ = watcher_handle.join();

    Ok(())
}

fn check_env(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("The path {} does not exist.", path))
    }
}
