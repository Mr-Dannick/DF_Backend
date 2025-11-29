use dotenv::dotenv;
mod kill_watcher;
mod database_setup;
use std::env;
use std::path::Path;
use crate::database_setup::setup_database;

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

    kill_watcher::watch_console_log(file_path, timeout);
    Ok(())
}

fn check_env(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("The path {} does not exist.", path))
    }
}
