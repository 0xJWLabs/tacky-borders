#![allow(dead_code)]
extern crate windows;

use anyhow::Result as AnyResult;
use std::fmt::Debug;
use std::io::Result as IoResult;
use win_open::Result as WinOpenResult;
use windows::core::Result as WinResult;

pub trait LogIfErr {
    fn log_if_err(&self);
    fn log_if_err_message_pretty(&self, message: &str, unknown: bool);
    fn log_if_err_message(&self, message: &str, unknown: bool);
    fn map_err_with_log(self) -> Self;
    fn map_err_with_log_pretty(self) -> Self;
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

    fn log_if_err_message_pretty(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:#?}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn log_if_err_message(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:?}");
            } else {
                error!("{message}: {e}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
    }

    fn map_err_with_log_pretty(self) -> Self {
        self.map_err(|err| {
            error!("{err:#?}");
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

    fn log_if_err_message_pretty(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:#?}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn log_if_err_message(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:?}");
            } else {
                error!("{message}: {e}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
    }

    fn map_err_with_log_pretty(self) -> Self {
        self.map_err(|err| {
            error!("{err:#?}");
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

    fn log_if_err_message_pretty(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:#?}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn log_if_err_message(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:?}");
            } else {
                error!("{message}: {e}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
    }

    fn map_err_with_log_pretty(self) -> Self {
        self.map_err(|err| {
            error!("{err:#?}");
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

    fn log_if_err_message_pretty(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:#?}");
            } else {
                error!("{message}: {e:#}");
            }
        }
    }

    fn log_if_err_message(&self, message: &str, unknown: bool) {
        if let Err(e) = self {
            if unknown {
                error!("{message}: {e:?}");
            } else {
                error!("{message}: {e}");
            }
        }
    }

    fn map_err_with_log(self) -> Self {
        self.map_err(|err| {
            error!("{err:?}");
            err
        })
    }

    fn map_err_with_log_pretty(self) -> Self {
        self.map_err(|err| {
            error!("{err:#?}");
            err
        })
    }
}
