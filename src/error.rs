extern crate windows;

use anyhow::Result as AnyResult;
use std::fmt::Debug;
use std::io::Result as IoResult;
use win_open::Result as WinOpenResult;
use windows::core::Result as WinResult;

pub trait LogIfErr {
    fn log_if_err(&self);
    fn log_if_err_message(&self, message: &str, pretty: bool);
    fn map_err_with_log(self) -> Self;
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

    fn log_if_err_message(&self, message: &str, pretty: bool) {
        if let Err(e) = self {
            if pretty {
                error!("{message}: {e}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
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

    fn log_if_err_message(&self, message: &str, pretty: bool) {
        if let Err(e) = self {
            if pretty {
                error!("{message}: {e}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
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

    fn log_if_err_message(&self, message: &str, pretty: bool) {
        if let Err(e) = self {
            if pretty {
                error!("{message}: {e}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
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

    fn log_if_err_message(&self, message: &str, pretty: bool) {
        if let Err(e) = self {
            if pretty {
                error!("{message}: {e}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
    }
}
