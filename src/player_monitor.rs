use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::thread;
use std::time::Duration;
use regex::Regex;
use std::env;

// Add mysql imports
use mysql::{Pool, prelude::*};

#[derive(Debug)]
pub struct PlayerConnection {
    pub identity: String,
    pub ip_address: String,
    pub reforger_id: String,
    pub username: String,
    pub battleye_guid: String,
}

pub struct PlayerMonitor {
    log_path: String,
    file_position: u64,
}

impl PlayerMonitor {
    pub fn new(log_path: &str) -> Self {
        Self {
            log_path: log_path.to_string(),
            file_position: 0,
        }
    }

    // Changed to take ownership so the monitor can be moved into a thread.
    pub fn start_monitoring(mut self) {
        println!("Starting player connection monitor...");

        // Try to create a DB pool from env vars. If any are missing or the pool fails,
        // continue running but skip DB writes.
        let db_pool: Option<Pool> = match (
            env::var("DATABASE_USER"),
            env::var("DATABASE_PASSWORD"),
            env::var("DATABASE_IP"),
            env::var("DATABASE_PORT"),
            env::var("DATABASE_NAME"),
        ) {
            (Ok(user), Ok(pass), Ok(ip), Ok(port), Ok(db)) => {
                let url = format!("mysql://{}:{}@{}:{}/{}", user, pass, ip, port, db);
                match Pool::new(url.as_str()) {
                    Ok(p) => {
                        println!("DB pool created");
                        Some(p)
                    }
                    Err(e) => {
                        eprintln!("Failed to create DB pool: {}", e);
                        None
                    }
                }
            }
            _ => {
                eprintln!("DB env vars missing; database writes disabled");
                None
            }
        };

        loop {
            if let Ok(connections) = self.check_for_events() {
                for player in connections {
                    println!("New player connected:");
                    println!("  Username: {}", player.username);
                    println!("  IP: {}", player.ip_address);
                    println!("  Reforger ID: {}", player.reforger_id);
                    println!("  BattlEye GUID: {}", player.battleye_guid);
                    println!("  Identity: {}", player.identity);
                    println!();

                    // If DB pool is available, attempt to upsert into Players, PlayerNames, ConnectionLogs
                    if let Some(pool) = &db_pool {
                        match pool.get_conn() {
                            Ok(mut conn) => {
                                // Upsert Players (using reforger_id unique constraint)
                                let upsert_players = r"INSERT INTO Players (reforger_id, battleye_guid)
                                    VALUES (?, ?)
                                    ON DUPLICATE KEY UPDATE
                                        battleye_guid = VALUES(battleye_guid),
                                        last_seen = CURRENT_TIMESTAMP";
                                if let Err(e) = conn.exec_drop(
                                    upsert_players,
                                    (player.reforger_id.as_str(), player.battleye_guid.as_str()),
                                ) {
                                    eprintln!("Failed to upsert Players: {}", e);
                                    continue;
                                }

                                // Get player_id
                                let select_id = "SELECT player_id FROM Players WHERE reforger_id = ?";
                                let player_id_res: Result<Option<u64>, _> =
                                    conn.exec_first(select_id, (player.reforger_id.as_str(),));
                                let player_id = match player_id_res {
                                    Ok(Some(id)) => id,
                                    Ok(None) => {
                                        eprintln!("Inserted player but could not retrieve id");
                                        continue;
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to query player id: {}", e);
                                        continue;
                                    }
                                };

                                // Upsert PlayerNames (unique (player_id, username))
                                let upsert_name = r"INSERT INTO PlayerNames (player_id, username)
                                    VALUES (?, ?)
                                    ON DUPLICATE KEY UPDATE
                                        last_used = CURRENT_TIMESTAMP";
                                if let Err(e) = conn.exec_drop(
                                    upsert_name,
                                    (player_id, player.username.as_str()),
                                ) {
                                    eprintln!("Failed to upsert PlayerNames: {}", e);
                                    // continue to connection logs attempt anyway
                                }

                                // Upsert ConnectionLogs (primary key (player_id, ip_address))
                                let upsert_conn = r"INSERT INTO ConnectionLogs (player_id, ip_address, username, connected_at)
                                    VALUES (?, ?, ?, CURRENT_TIMESTAMP)
                                    ON DUPLICATE KEY UPDATE
                                        username = VALUES(username),
                                        connected_at = CURRENT_TIMESTAMP";
                                if let Err(e) = conn.exec_drop(
                                    upsert_conn,
                                    (player_id, player.ip_address.as_str(), player.username.as_str()),
                                ) {
                                    eprintln!("Failed to upsert ConnectionLogs: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to get DB connection from pool: {}", e);
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_secs(10));
        }
    }

    fn check_for_events(&mut self) -> Result<Vec<PlayerConnection>, std::io::Error> {
        let path = Path::new(&self.log_path);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(self.file_position))?;

        let reader = BufReader::new(file);
        let mut lines: Vec<String> = Vec::new();

        for line in reader.lines() {
            if let Ok(content) = line {
                self.file_position += content.len() as u64 + 1;
                lines.push(content);
            }
        }

        let connections = self.parse_player_connections(&lines);

        Ok(connections)
    }

    fn parse_player_connections(&self, lines: &[String]) -> Vec<PlayerConnection> {
        let identity_regex = Regex::new(r"identity=(\w+).*address=([0-9.]+)").unwrap();
        let auth_regex = Regex::new(r"identityId=([a-f0-9-]+)\s+name=(\w+)").unwrap();
        let guid_regex = Regex::new(r"BE GUID:\s+(\w+)").unwrap();

        let mut connections = Vec::new();
        let mut current_identity = None;
        let mut current_ip = None;
        let mut current_reforger_id = None;
        let mut current_username = None;

        for line in lines {
            if line.contains("authenticating") {
                if let Some(caps) = identity_regex.captures(line) {
                    current_identity = Some(caps[1].to_string());
                    current_ip = Some(caps[2].to_string());
                }
            }

            if line.contains("Authenticated player") {
                if let Some(caps) = auth_regex.captures(line) {
                    current_reforger_id = Some(caps[1].to_string());
                    current_username = Some(caps[2].to_string());
                }
            }

            if line.contains("BE GUID:") {
                if let Some(caps) = guid_regex.captures(line) {
                    if let (Some(id), Some(ip), Some(rid), Some(user)) =
                        (&current_identity, &current_ip, &current_reforger_id, &current_username) {
                        connections.push(PlayerConnection {
                            identity: id.clone(),
                            ip_address: ip.clone(),
                            reforger_id: rid.clone(),
                            username: user.clone(),
                            battleye_guid: caps[1].to_string(),
                        });

                        current_identity = None;
                        current_ip = None;
                        current_reforger_id = None;
                        current_username = None;
                    }
                }
            }
        }

        connections
    }
}
