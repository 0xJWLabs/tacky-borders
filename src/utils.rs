use anyhow::Context;
use anyhow::Error;
use anyhow::Result as AnyResult;
use std::fs::exists;
use std::fs::write;
use std::fs::File;
use std::fs::OpenOptions;

use crate::border_config;

// Log File
pub fn get_log() -> AnyResult<File, Error> {
    let log_dir = border_config::Config::get_config_dir()?;
    let log_path = log_dir.join("log.txt");

    if !exists(&log_path).context("Could not find log file")? {
        write(&log_path, "").context("could not generate log file")?;
    }

    let _ = write(&log_path, "");

    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_path)
        .context(format!("Failed to open log file: {:?}", log_path))?;

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
