use anyhow::Context;
use anyhow::Result as AnyResult;
use std::ffi::OsString;
use std::io::Result as IoResult;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::core::Result as WinResult;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::FOLDERID_Profile;
use windows::Win32::UI::Shell::SHGetKnownFolderPath;
use windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG;

pub trait LogIfErr {
    fn log_if_err(&self);
}

impl<T> LogIfErr for AnyResult<T>
where
    T: std::fmt::Debug, // Ensuring T implements Debug so we can log it
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}"); // Log error using Debug formatting
        }
    }
}

impl<T> LogIfErr for WinResult<T>
where
    T: std::fmt::Debug, // Ensuring T implements Debug so we can log it
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}"); // Log error using Debug formatting
        }
    }
}

impl<T> LogIfErr for IoResult<T>
where
    T: std::fmt::Debug,
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}");
        }
    }
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
