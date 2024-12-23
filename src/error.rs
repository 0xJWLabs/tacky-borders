extern crate windows;

use anyhow::Result as AnyResult;
use std::fmt::Debug;
use std::io::Result as IoResult;
use win_open::Result as WinOpenResult;
use windows::core::Result as WinResult;

pub trait LogIfErr {
    fn log_if_err(&self);
}

impl<T> LogIfErr for AnyResult<T>
where
    T: Debug, // Ensuring T implements Debug so we can log it
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}"); // Log error using Debug formatting
        }
    }
}

impl<T> LogIfErr for WinResult<T>
where
    T: Debug, // Ensuring T implements Debug so we can log it
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}"); // Log error using Debug formatting
        }
    }
}

impl<T> LogIfErr for IoResult<T>
where
    T: Debug,
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}");
        }
    }
}

impl<T> LogIfErr for WinOpenResult<T>
where
    T: Debug,
{
    fn log_if_err(&self) {
        if let Err(e) = self {
            error!("{e:#}");
        }
    }
}
