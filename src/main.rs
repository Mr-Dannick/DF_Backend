use dotenv::dotenv;
use std::env;
use std::path::Path;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); // Reads the .env file

    let server_path = env::var("SERVER_PATH")?;

    let file_path = Path::new(&server_path).join("console.log");

    // Check of het path bestaat
    if check_env(file_path.to_str().unwrap()).is_ok() {
        println!("The path exists: {}", file_path.display());
    } else {
        println!("The path does not exist: {}", file_path.display());
        panic!("Exiting due to wrong server path in .env.");
    }

    Ok(())
}

fn check_env(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("The path {} does not exist.", path))
    }
}