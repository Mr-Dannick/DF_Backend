# DF Backend

A Rust-based backend service for monitoring and tracking game server statistics. This application monitors player connections and kills from a game server's console log and stores the data in a MySQL database.

## Features

- **Player Connection Monitoring**: Tracks player connections including usernames, IP addresses, Reforger IDs, and BattlEye GUIDs
- **Kill Event Tracking**: Parses and records player kills with detailed information:
  - Killer and victim names
  - Weapon used
  - Kill distance
  - Team kill detection
  - Faction information
- **Comprehensive Statistics**: Maintains player statistics including:
  - Kill/Death ratios
  - Weapon usage stats
  - Player vs Player records
  - Longest kills

## Requirements

- Rust (2024 edition)
- MySQL database server

## Dependencies

- `dotenv` - Environment variable management
- `mysql` - MySQL database driver
- `regex` - Log parsing
- `uuid` - Unique identifier generation

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/Mr-Dannick/DF_Backend.git
   cd DF_Backend
   ```

2. Copy the environment example file and configure your settings:
   ```bash
   cp .env_example .env
   ```

3. Edit `.env` with your configuration:
   ```
   SERVER_PATH=./
   PLAYER_KILL_CHECKER_TIMEOUT=10
   DATABASE_IP=127.0.0.1
   DATABASE_PORT=3306
   DATABASE_NAME=your_database_name
   DATABASE_USER=your_username
   DATABASE_PASSWORD=your_password
   DATABASE_SETUP_COMPLETE=false
   ```

4. Build the project:
   ```bash
   cargo build --release
   ```

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_PATH` | Path to the game server directory containing console.log | `./` |
| `PLAYER_KILL_CHECKER_TIMEOUT` | Interval in seconds to check for new events | `10` |
| `DATABASE_IP` | MySQL server IP address | `127.0.0.1` |
| `DATABASE_PORT` | MySQL server port | `3306` |
| `DATABASE_NAME` | Name of the database | - |
| `DATABASE_USER` | Database username | - |
| `DATABASE_PASSWORD` | Database password | - |
| `DATABASE_SETUP_COMPLETE` | Whether database tables have been created | `false` |

## Usage

Run the application:
```bash
cargo run --release
```

On first run with `DATABASE_SETUP_COMPLETE=false`, the application will automatically create the required database tables:

- `Players` - Core player records
- `PlayerNames` - Player username history
- `ConnectionLogs` - Player connection history
- `PlayerKills` - Individual kill records
- `PlayerWeaponStats` - Weapon usage statistics per player
- `PlayerVsPlayerStats` - Player vs player kill statistics
- `PlayerStats` - Aggregated player statistics

## Database Schema

The application creates and maintains the following tables:

### Players
Stores core player information with unique Reforger IDs and optional BattlEye GUIDs.

### PlayerNames
Tracks username history for each player.

### ConnectionLogs
Records player connections with IP addresses and timestamps.

### PlayerKills
Logs individual kill events with weapon, distance, faction, and team kill information.

### PlayerWeaponStats
Aggregates weapon usage per player including total kills, team kills, and longest kill distance.

### PlayerVsPlayerStats
Tracks kill statistics between specific player pairs.

### PlayerStats
Maintains overall player statistics including K/D ratio and favorite weapon.

## License

This project is open source.
