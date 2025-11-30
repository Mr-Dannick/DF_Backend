// rust
// File: `src/kill_watcher.rs`
use mysql::{params, prelude::*, Pool};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

pub fn watch_console_log(file_path: PathBuf, timeout: u64) {
    println!("Watching file: {}", file_path.display());
    println!("Checking for PLAYER_KILLED events every {} seconds...\n", timeout);

    let pool = init_db_pool();

    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut last_position = file.seek(SeekFrom::End(0)).expect("Failed to seek");

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

        for line in reader.lines() {
            if let Ok(line) = line {
                // Do not reset position based on "Game successfully created." anymore.
                if line.contains("PLAYER_KILLED:") {
                    if let Some(kill) = parse_kill_line(&line) {
                        print_kill(&kill);
                        if let Some(ref pool) = pool {
                            if let Err(e) = persist_kill(pool, &kill) {
                                eprintln!("DB error persisting kill: {}", e);
                            }
                        } else {
                            eprintln!("DB pool not initialized; skipping DB write.");
                        }
                    }
                }
            }
        }

        // Update last_position to current EOF so we only read new content next time
        last_position = file.seek(SeekFrom::End(0)).expect("Failed to seek end");
    }
}

fn init_db_pool() -> Option<Pool> {
    let database_user = env::var("DATABASE_USER").ok()?;
    let database_password = env::var("DATABASE_PASSWORD").ok()?;
    let database_ip = env::var("DATABASE_IP").ok()?;
    let database_port = env::var("DATABASE_PORT").ok()?;
    let database_name = env::var("DATABASE_NAME").ok()?;

    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        database_user, database_password, database_ip, database_port, database_name
    );

    match Pool::new(url.as_str()) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Failed to create DB pool: {}", e);
            None
        }
    }
}

#[derive(Debug)]
struct KillEvent {
    killer_name: String,
    victim_name: String,
    weapon: Option<String>,
    distance: Option<f64>,
    is_team_kill: bool,
    killer_faction: Option<String>,
    victim_faction: Option<String>,
}

fn parse_kill_line(line: &str) -> Option<KillEvent> {
    // find data after "PLAYER_KILLED:"
    let marker = "PLAYER_KILLED:";
    let start = line.find(marker)? + marker.len();
    let data = &line[start..];

    let mut killer_name = None;
    let mut victim_name = None;
    let mut weapon = None;
    let mut distance = None;
    let mut is_team_kill = false;
    let mut killer_faction = None;
    let mut victim_faction = None;

    for pair in data.split(", ") {
        if let Some((k, v)) = pair.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('\'');
            match key {
                "killerName" => killer_name = Some(val.to_string()),
                "victimName" => victim_name = Some(val.to_string()),
                "weaponName" => weapon = Some(val.to_string()),
                "killDistance" => distance = val.parse::<f64>().ok(),
                "isTeamKill" => {
                    is_team_kill = matches!(val.to_lowercase().as_str(), "true" | "1")
                }
                "killerFaction" => killer_faction = Some(normalize_faction(val).to_string()),
                "victimFaction" => victim_faction = Some(normalize_faction(val).to_string()),
                _ => {}
            }
        }
    }

    Some(KillEvent {
        killer_name: killer_name?,
        victim_name: victim_name?,
        weapon,
        distance,
        is_team_kill,
        killer_faction,
        victim_faction,
    })
}

fn print_kill(k: &KillEvent) {
    println!("=== PLAYER KILLED ===");
    println!("Killer: {}", k.killer_name);
    println!("Victim: {}", k.victim_name);
    if let Some(ref w) = k.weapon {
        println!("Weapon: {}", w);
    }
    if let Some(d) = k.distance {
        println!("Distance: {} meters", d);
    }
    println!("Team Kill: {}", k.is_team_kill);
    if let Some(ref f) = k.killer_faction {
        println!("Killer Faction: {}", f);
    }
    if let Some(ref f) = k.victim_faction {
        println!("Victim Faction: {}", f);
    }
    println!();
}

fn persist_kill(pool: &Pool, k: &KillEvent) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get_conn()?;

    // ensure killer player exists -> player_id
    let killer_id = get_or_create_player(&mut conn, &k.killer_name)?;
    let victim_id = get_or_create_player(&mut conn, &k.victim_name)?;

    // Insert into PlayerKills (killed_at uses NOW())
    conn.exec_drop(
        r"INSERT INTO PlayerKills
        (killer_id, victim_id, weapon, distance, is_team_kill, killer_faction, victim_faction, killed_at)
        VALUES (:killer, :victim, :weapon, :distance, :is_team_kill, :kf, :vf, NOW())",
        params! {
            "killer" => killer_id,
            "victim" => victim_id,
            "weapon" => k.weapon.as_deref().unwrap_or(""),
            "distance" => k.distance,
            "is_team_kill" => k.is_team_kill,
            "kf" => k.killer_faction.as_deref().unwrap_or(""),
            "vf" => k.victim_faction.as_deref().unwrap_or("")
        },
    )?;

    // Update PlayerWeaponStats for killer
    let distance_val = k.distance.unwrap_or(0.0);
    conn.exec_drop(
        r"INSERT INTO PlayerWeaponStats
        (player_id, weapon, total_kills, total_team_kills, total_distance, longest_kill, last_kill)
        VALUES (:pid, :weapon, 1, :tk, :dist, :dist, NOW())
        ON DUPLICATE KEY UPDATE
            total_kills = total_kills + 1,
            total_team_kills = total_team_kills + VALUES(total_team_kills),
            total_distance = total_distance + VALUES(total_distance),
            longest_kill = GREATEST(longest_kill, VALUES(longest_kill)),
            last_kill = VALUES(last_kill)",
        params! {
            "pid" => killer_id,
            "weapon" => k.weapon.as_deref().unwrap_or(""),
            "tk" => if k.is_team_kill { 1 } else { 0 },
            "dist" => distance_val
        },
    )?;

    // Update PlayerVsPlayerStats (killer -> victim)
    conn.exec_drop(
        r"INSERT INTO PlayerVsPlayerStats
        (killer_id, victim_id, total_kills, last_kill)
        VALUES (:killer, :victim, 1, NOW())
        ON DUPLICATE KEY UPDATE
            total_kills = total_kills + 1,
            last_kill = VALUES(last_kill)",
        params! { "killer" => killer_id, "victim" => victim_id },
    )?;

    // Update PlayerStats for killer (increment kills)
    conn.exec_drop(
        r"INSERT INTO PlayerStats
        (player_id, total_kills, total_deaths, total_team_kills, longest_kill)
        VALUES (:pid, 1, 0, :tk, :dist)
        ON DUPLICATE KEY UPDATE
            total_kills = total_kills + 1,
            total_team_kills = total_team_kills + VALUES(total_team_kills),
            longest_kill = GREATEST(longest_kill, VALUES(longest_kill))",
        params! { "pid" => killer_id, "tk" => if k.is_team_kill { 1 } else { 0 }, "dist" => distance_val },
    )?;

    // Update PlayerStats for victim (increment deaths)
    conn.exec_drop(
        r"INSERT INTO PlayerStats
        (player_id, total_kills, total_deaths, total_team_kills)
        VALUES (:pid, 0, 1, 0)
        ON DUPLICATE KEY UPDATE
            total_deaths = total_deaths + 1",
        params! { "pid" => victim_id },
    )?;

    // Recompute kd_ratio for killer and victim
    for pid in &[killer_id, victim_id] {
        conn.exec_drop(
            r"UPDATE PlayerStats
            SET kd_ratio = CASE WHEN total_deaths = 0 THEN total_kills ELSE total_kills / total_deaths END
            WHERE player_id = :pid",
            params! { "pid" => pid },
        )?;
    }

    Ok(())
}

fn get_or_create_player(conn: &mut mysql::PooledConn, username: &str) -> Result<u64, Box<dyn std::error::Error>> {
    // try find in PlayerNames
    if let Some(row) = conn.exec_first::<(u64,), _, _>(
        "SELECT player_id FROM PlayerNames WHERE username = :u LIMIT 1",
        params! { "u" => username },
    )? {
        return Ok(row.0);
    }

    // not found -> create Player with generated reforger_id and create PlayerNames
    let reforger_id = Uuid::new_v4().to_string();
    conn.exec_drop(
        "INSERT INTO Players (reforger_id) VALUES (:rid)",
        params! { "rid" => &reforger_id },
    )?;
    let player_id = conn.last_insert_id();
    conn.exec_drop(
        "INSERT INTO PlayerNames (player_id, username) VALUES (:pid, :uname)",
        params! { "pid" => player_id, "uname" => username },
    )?;

    // Also ensure PlayerStats row exists (so later updates work)
    conn.exec_drop(
        "INSERT INTO PlayerStats (player_id) VALUES (:pid) ON DUPLICATE KEY UPDATE player_id = player_id",
        params! { "pid" => player_id },
    )?;

    Ok(player_id)
}

fn normalize_faction(value: &str) -> &str {
    match value {
        "#WCS-Faction_NATO" => "NATO",
        "#WCS-Faction_RU" => "RU",
        other => other,
    }
}