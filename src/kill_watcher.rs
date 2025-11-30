// rust
// File: `src/kill_watcher.rs`
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub fn watch_console_log(file_path: PathBuf, timeout: u64) {
    println!("Watching file: {}", file_path.display());
    println!("Checking for PLAYER_KILLED events every {} seconds...\n", timeout);

    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut last_position = file.seek(SeekFrom::End(0)).expect("Failed to seek");

    // Remember the last observed "Game successfully created." time as milliseconds since midnight.
    let mut last_restart_ms: Option<u32> = None;

    loop {
        thread::sleep(Duration::from_secs(timeout));

        // Reopen file to handle rotation/truncation
        file = File::open(&file_path).expect("Failed to reopen file");

        // If file was truncated or recreated with smaller length, reset position
        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if file_len < last_position {
            println!(
                "Log file was truncated or rotated (len {} < last_position {}). Resetting last_position to 0.",
                file_len, last_position
            );
            last_position = 0;
        }

        file.seek(SeekFrom::Start(last_position)).expect("Failed to seek");

        let reader = BufReader::new(&file);
        let mut restart_detected = false;

        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("Game successfully created.") {
                    if let Some(parsed_ms) = parse_timestamp_ms(&line) {
                        match last_restart_ms {
                            None => {
                                println!(
                                    "Detected server start at {}. Forgetting last read position.",
                                    format_time_ms(parsed_ms)
                                );
                                last_restart_ms = Some(parsed_ms);
                                restart_detected = true;
                                break;
                            }
                            Some(prev_ms) => {
                                // Update only when it's a new start time:
                                // - a later time on same day, or
                                // - wrapped-around time (new day)
                                if parsed_ms != prev_ms {
                                    if parsed_ms > prev_ms || parsed_ms < prev_ms {
                                        println!(
                                            "Detected new server start (previous {} -> now {}). Resetting read position.",
                                            format_time_ms(prev_ms),
                                            format_time_ms(parsed_ms)
                                        );
                                        last_restart_ms = Some(parsed_ms);
                                        restart_detected = true;
                                        break;
                                    }
                                } else {
                                    // same timestamp as before -> ignore to avoid repeated resets
                                }
                            }
                        }
                    } else {
                        // couldn't parse timestamp; ignore to avoid spurious resets
                        eprintln!("Warning: failed to parse timestamp from server start line, ignoring.");
                    }
                }

                if line.contains("PLAYER_KILLED:") {
                    parse_and_print_kill(&line);
                }
            }
        }

        if restart_detected {
            last_position = 0;
            // next iteration will reopen and read from start
            continue;
        }

        // Update last_position to current EOF so we only read new content next time
        last_position = file.seek(SeekFrom::End(0)).expect("Failed to seek end");
    }
}

fn parse_timestamp_ms(line: &str) -> Option<u32> {
    // Expect leading time like "10:28:58.447 ..." - extract up to first space
    let first_space = line.find(' ')?;
    let time_str = &line[..first_space];

    // Split into h:m:s and milliseconds (optional)
    let (hms_part, ms_part) = if let Some(dot_idx) = time_str.find('.') {
        (&time_str[..dot_idx], &time_str[dot_idx + 1..])
    } else {
        (time_str, "0")
    };

    let mut hms_iter = hms_part.split(':');
    let hour: u32 = hms_iter.next()?.parse().ok()?;
    let minute: u32 = hms_iter.next()?.parse().ok()?;
    let second: u32 = hms_iter.next()?.parse().ok()?;
    // milliseconds may be 1-3 digits, normalize to milliseconds value (e.g., "447" -> 447)
    let ms: u32 = {
        let s = ms_part;
        // trim any non-digit just in case
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            0
        } else {
            digits.parse().ok()?
        }
    };

    Some(hour * 3_600_000 + minute * 60_000 + second * 1_000 + ms)
}

fn format_time_ms(ms: u32) -> String {
    let hour = ms / 3_600_000;
    let rem = ms % 3_600_000;
    let minute = rem / 60_000;
    let rem = rem % 60_000;
    let second = rem / 1000;
    let milli = rem % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hour, minute, second, milli)
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
                    "killerFaction" => {
                        println!("Killer Faction: {}", normalize_faction(cleaned_value))
                    }
                    "victimFaction" => {
                        println!("Victim Faction: {}", normalize_faction(cleaned_value))
                    }
                    _ => {}
                }
            }
        }
    }

    println!();
}
