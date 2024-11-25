use dirs::home_dir;
use std::fs;
use std::fs::write;
use std::fs::DirBuilder;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use windows::core::Result;

// Configuration
pub fn get_config() -> PathBuf {
    let home_dir = home_dir().expect("can't find home path");
    let config_dir = home_dir.join(".config").join("tacky-borders");
    let fallback_dir = home_dir.join(".tacky-borders");

    let dir_path = if has_file(&config_dir).expect("Couldn't check if config dir exists") {
        config_dir
    } else if has_file(&fallback_dir).expect("Couldn't check if config dir exists") {
        fallback_dir
    } else {
        DirBuilder::new()
            .recursive(true)
            .create(&config_dir)
            .expect("could not create config directory!");

        config_dir
    };

    dir_path
}

// Log File
pub fn get_log() -> Result<File> {
    let config_dir = get_config();
    let log_path = config_dir.join("log.txt");

    if !has_file(&log_path).expect("Couldn't check if log path exists") {
        // Overwrite the file with an empty string
        write(&log_path, "").map_err(|e| {
            eprintln!("Failed to reset log file: {:?}", e);
            e
        })?;
    }

    if !has_file(&log_path).expect("Couldn't check if log path exists") {
        write(&log_path, "").expect("could not generate log.txt");
    }

    let file = OpenOptions::new()
        .append(true) // Allow appending to the file
        .create(true) // Create the file if it doesn't exist
        .open(&log_path)
        .map_err(|err| {
            error!("{}", &format!("Failed to open log file: {:?}", &log_path),);
            debug!("{}", &format!("{:?}", err),);
            err
        })?;

    Ok(file)
}

pub fn strip_string(input: String, prefixes: &[&str], suffix: char) -> String {
    let mut result = input;

    // Remove matching prefix (if any)
    for &prefix in prefixes {
        if let Some(stripped) = result.strip_prefix(prefix) {
            result = stripped.to_string();
            break; // Only remove the first matching prefix
        }
    }

    // Remove suffix (if it exists)
    result.strip_suffix(suffix).unwrap_or(&result).to_string()
}

pub fn has_file<P>(path: P) -> io::Result<bool>
where
    P: AsRef<Path>,
{
    #[cfg(feature = "rust150")]
    {
        // Code for Rust 1.7.0
        Ok(fs::metadata(path.as_ref()).is_ok())
    }

    #[cfg(feature = "rust180")]
    {
        // Code for Rust 1.8.0 or greater
        path.as_ref().try_exists()
    }

    #[cfg(not(any(feature = "rust150", feature = "rust180")))]
    {
        fs::exists(path.as_ref())
    }
}
