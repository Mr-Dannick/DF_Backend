use dotenv::dotenv;
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let server_path = env::var("SERVER_PATH")?;
    let file_path = Path::new(&server_path).join("console.log");

    check_env(file_path.to_str().unwrap())?;

    println!("Watching file: {}", file_path.display());
    println!("Checking for PLAYER_KILLED events every 10 seconds...\n");

    let mut file = File::open(&file_path)?;
    let mut last_position = file.seek(SeekFrom::End(0))?;

    loop {
        thread::sleep(Duration::from_secs(10));

        file = File::open(&file_path)?;
        file.seek(SeekFrom::Start(last_position))?;

        let reader = BufReader::new(&file);

        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("PLAYER_KILLED:") {
                    parse_and_print_kill(&line);
                }
            }
        }

        last_position = file.seek(SeekFrom::End(0))?;
    }
}

fn parse_and_print_kill(line: &str) {
    println!("=== PLAYER KILLED ===");

    // Find the second [...]
    if let Some(first_bracket) = line.find(']') {
        let remaining = &line[first_bracket + 1..];
        if let Some(second_open) = remaining.find('[') {
            if let Some(second_close) = remaining.find(']') {
                let timestamp = &remaining[second_open + 1..second_close];
                println!("Time: {}", timestamp);
            }
        }
    }

    // Extract key-value pairs
    if let Some(data_start) = line.find("PLAYER_KILLED:") {
        let data = &line[data_start + 14..];

        for pair in data.split(", ") {
            if let Some((key, value)) = pair.split_once('=') {
                let cleaned_value = value.trim_matches('\'');
                match key.trim() {
                    "killerName" => println!("Killer: {}", cleaned_value),
                    "victimName" => println!("Victim: {}", cleaned_value),
                    "weaponName" => println!("Weapon: {}", cleaned_value),
                    "killDistance" => println!("Distance: {} meters", cleaned_value),
                    "isTeamKill" => println!("Team Kill: {}", cleaned_value),
                    "killerFaction" => println!("Killer Faction: {}", cleaned_value),
                    "victimFaction" => println!("Victim Faction: {}", cleaned_value),
                    _ => {}
                }
            }
        }
    }

    println!();
}

fn check_env(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("The path {} does not exist.", path))
    }
}