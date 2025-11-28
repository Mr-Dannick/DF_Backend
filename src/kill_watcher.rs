use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::thread;
use std::time::Duration;
use std::path::PathBuf;

pub fn watch_console_log(file_path: PathBuf, timeout: u64) {
    println!("Watching file: {}", file_path.display());
    println!("Checking for PLAYER_KILLED events every {} seconds...\n", timeout);

    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut last_position = file.seek(SeekFrom::End(0)).expect("Failed to seek");

    loop {
        thread::sleep(Duration::from_secs(timeout));

        file = File::open(&file_path).expect("Failed to reopen file");
        file.seek(SeekFrom::Start(last_position)).expect("Failed to seek");

        let reader = BufReader::new(&file);

        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("PLAYER_KILLED:") {
                    parse_and_print_kill(&line);
                }
            }
        }

        last_position = file.seek(SeekFrom::End(0)).expect("Failed to seek end");
    }
}
fn normalize_faction(value: &str) -> &str {
    match value {
        "#WCS-Faction_NATO" => "NATO",
        "#WCS-Faction_RU" => "RU",
        other => other,
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
                    "killerFaction" => println!("Killer Faction: {}", normalize_faction (cleaned_value)),
                    "victimFaction" => println!("Victim Faction: {}",normalize_faction (cleaned_value)),
                    _ => {}
                }
            }
        }
    }

    println!();
}