use mysql::*;
use mysql::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn setup_database(
    database_ip: &str,
    database_port: &str,
    database_name: &str,
    database_user: &str,
    database_password: &str,
) {
    println!("Setting up the database...");

    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        database_user, database_password, database_ip, database_port, database_name
    );

    match Pool::new(url.as_str()) {
        Ok(pool) => match pool.get_conn() {
            Ok(mut conn) => {
                println!("Connected to database {}", database_name);

                // Drop tables if they exist (in correct order due to foreign keys)
                let _ = conn.query_drop("DROP TABLE IF EXISTS PlayerKills");
                let _ = conn.query_drop("DROP TABLE IF EXISTS PlayerWeaponStats");
                let _ = conn.query_drop("DROP TABLE IF EXISTS PlayerVsPlayerStats");
                let _ = conn.query_drop("DROP TABLE IF EXISTS PlayerStats");
                let _ = conn.query_drop("DROP TABLE IF EXISTS ConnectionLogs");
                let _ = conn.query_drop("DROP TABLE IF EXISTS PlayerNames");
                let _ = conn.query_drop("DROP TABLE IF EXISTS Players");

                // Create Players table
                let _ = conn.query_drop(
                    r"CREATE TABLE Players (
                        player_id INT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
                        reforger_id CHAR(36) UNIQUE NOT NULL,
                        battleye_guid CHAR(32),
                        first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                        last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                        INDEX idx_reforger_id (reforger_id)
                    )"
                );

                // Create PlayerNames table
                let _ = conn.query_drop(
                    r"CREATE TABLE PlayerNames (
                        name_id INT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
                        player_id INT UNSIGNED NOT NULL,
                        username VARCHAR(255) NOT NULL,
                        first_used DATETIME DEFAULT CURRENT_TIMESTAMP,
                        last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                        FOREIGN KEY (player_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        INDEX idx_player_id (player_id),
                        INDEX idx_username (username),
                        UNIQUE KEY unique_player_username (player_id, username)
                    )"
                );

                // Create ConnectionLogs table
                let _ = conn.query_drop(
                    r"CREATE TABLE ConnectionLogs (
                        player_id INT UNSIGNED NOT NULL,
                        ip_address VARCHAR(45) NOT NULL,
                        username VARCHAR(255) NOT NULL,
                        connected_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                        PRIMARY KEY (player_id, ip_address),
                        FOREIGN KEY (player_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        INDEX idx_ip_address (ip_address),
                        INDEX idx_connected_at (connected_at)
                    )"
                );

                // Create PlayerKills table
                let _ = conn.query_drop(
                    r"CREATE TABLE PlayerKills (
                        kill_id INT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
                        killer_id INT UNSIGNED NOT NULL,
                        victim_id INT UNSIGNED NOT NULL,
                        weapon VARCHAR(100) NOT NULL,
                        distance DECIMAL(8,4),
                        is_team_kill BOOLEAN DEFAULT FALSE,
                        killer_faction VARCHAR(50),
                        victim_faction VARCHAR(50),
                        killed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                        FOREIGN KEY (killer_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        FOREIGN KEY (victim_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        INDEX idx_killer_id (killer_id),
                        INDEX idx_victim_id (victim_id),
                        INDEX idx_weapon (weapon),
                        INDEX idx_killed_at (killed_at)
                    )"
                );

                // Create PlayerWeaponStats table
                let _ = conn.query_drop(
                    r"CREATE TABLE PlayerWeaponStats (
                        player_id INT UNSIGNED NOT NULL,
                        weapon VARCHAR(100) NOT NULL,
                        total_kills INT UNSIGNED DEFAULT 0,
                        total_team_kills INT UNSIGNED DEFAULT 0,
                        total_distance DECIMAL(12,4) DEFAULT 0,
                        longest_kill DECIMAL(8,4) DEFAULT 0,
                        last_kill DATETIME,
                        PRIMARY KEY (player_id, weapon),
                        FOREIGN KEY (player_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        INDEX idx_weapon (weapon),
                        INDEX idx_total_kills (total_kills)
                    )"
                );

                // Create PlayerVsPlayerStats table
                let _ = conn.query_drop(
                    r"CREATE TABLE PlayerVsPlayerStats (
                        killer_id INT UNSIGNED NOT NULL,
                        victim_id INT UNSIGNED NOT NULL,
                        total_kills INT UNSIGNED DEFAULT 0,
                        last_kill DATETIME,
                        PRIMARY KEY (killer_id, victim_id),
                        FOREIGN KEY (killer_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        FOREIGN KEY (victim_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        INDEX idx_killer_id (killer_id),
                        INDEX idx_victim_id (victim_id)
                    )"
                );

                // Create PlayerStats table
                let _ = conn.query_drop(
                    r"CREATE TABLE PlayerStats (
                        player_id INT UNSIGNED PRIMARY KEY,
                        total_kills INT UNSIGNED DEFAULT 0,
                        total_deaths INT UNSIGNED DEFAULT 0,
                        total_team_kills INT UNSIGNED DEFAULT 0,
                        kd_ratio DECIMAL(6,2) DEFAULT 0,
                        longest_kill DECIMAL(8,4) DEFAULT 0,
                        favorite_weapon VARCHAR(100),
                        FOREIGN KEY (player_id) REFERENCES Players(player_id) ON DELETE CASCADE,
                        INDEX idx_total_kills (total_kills),
                        INDEX idx_kd_ratio (kd_ratio)
                    )"
                );

                println!("Database tables created successfully");

                // On success, update .env to mark DATABASE_SETUP_COMPLETE=true
                if let Err(e) = set_dotenv_key("DATABASE_SETUP_COMPLETE", "true") {
                    eprintln!("Failed to update .env: {}", e);
                } else {
                    println!("Updated .env: DATABASE_SETUP_COMPLETE=true");
                }
            }
            Err(e) => eprintln!("Failed to get connection: {}", e),
        },
        Err(e) => eprintln!("Failed to create pool: {}", e),
    }
}

fn set_dotenv_key(key: &str, value: &str) -> Result<(), std::io::Error> {
    let mut dotenv_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    dotenv_path.push(".env");

    // Read existing .env (if any)
    let content = fs::read_to_string(&dotenv_path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let mut found = false;
    for line in lines.iter_mut() {
        // preserve comments and other lines; only replace exact key=... lines
        if line.starts_with(&format!("{}=", key)) {
            *line = format!("{}={}", key, value);
            found = true;
            break;
        }
    }

    if !found {
        // append key at end
        lines.push(format!("{}={}", key, value));
    }

    // Write back with Unix newlines (keeps file simple)
    fs::write(&dotenv_path, lines.join("\n"))?;
    Ok(())
}
