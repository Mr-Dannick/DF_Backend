use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::thread;
use std::time::Duration;
use regex::Regex;

#[derive(Debug)]
pub struct PlayerConnection {
    pub identity: String,
    pub ip_address: String,
    pub reforger_id: String,
    pub username: String,
    pub battleeye_guid: String,
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

        loop {
            if let Ok(connections) = self.check_for_events() {
                for player in connections {
                    println!("New player connected:");
                    println!("  Username: {}", player.username);
                    println!("  IP: {}", player.ip_address);
                    println!("  Reforger ID: {}", player.reforger_id);
                    println!("  BattlEye GUID: {}", player.battleeye_guid);
                    println!("  Identity: {}", player.identity);
                    println!();
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
                            battleeye_guid: caps[1].to_string(),
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
