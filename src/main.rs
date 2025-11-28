mod kill_watcher;

use dotenv::dotenv;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let server_path = env::var("SERVER_PATH")?;
    let timeout: u64 = env::var("PLAYER_KILL_CHECKER_TIMEOUT")?.parse()?;
    let file_path = Path::new(&server_path).join("console.log");

    check_env(file_path.to_str().unwrap())?;

    // Start de kill watcher
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