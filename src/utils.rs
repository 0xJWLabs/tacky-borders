use crate::border_config;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result as AnyResult;
use std::ffi::OsString;
use std::fs::exists;
use std::fs::write;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::FOLDERID_Profile;
use windows::Win32::UI::Shell::SHGetKnownFolderPath;
use windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG;

#[macro_export]
macro_rules! log_if_err {
    ($err:expr) => {
        if let Err(e) = $err {
            // TODO for some reason if I use {:#} or {:?}, some errors will repeatedly print (like
            // the one in main.rs for tray_icon_result). It could have something to do with how they
            // implement .source()
            error!("{e:#}");
        }
    };
}

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

pub fn home_dir() -> AnyResult<PathBuf> {
    unsafe {
        // Call SHGetKnownFolderPath with NULL token (default user)
        let path_ptr =
            SHGetKnownFolderPath(&FOLDERID_Profile, KNOWN_FOLDER_FLAG(0), HANDLE::default())
                .context("Failed to retrieve the home directory path")?;

        if path_ptr.0.is_null() {
            anyhow::bail!("SHGetKnownFolderPath returned a null pointer");
        }

        // Convert PWSTR to OsString
        let len = (0..).take_while(|&i| *path_ptr.0.add(i) != 0).count();
        let wide_slice = std::slice::from_raw_parts(path_ptr.0, len);
        let os_string = OsString::from_wide(wide_slice);

        // Free the memory allocated by SHGetKnownFolderPath
        CoTaskMemFree(Some(path_ptr.0 as *const _));

        // Return the PathBuf wrapped in Ok
        Ok(PathBuf::from(os_string))
    }
}
