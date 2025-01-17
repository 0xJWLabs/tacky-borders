#![allow(dead_code)]
extern crate windows;

use std::fmt::Debug;

pub trait LogIfErr {
    fn log_if_err(&self);
    fn log_if_err_message_pretty(&self, message: &str, unknown: bool);
    fn log_if_err_message(&self, message: &str, unknown: bool);
    fn map_err_with_log(self) -> Self;
    fn map_err_with_log_pretty(self) -> Self;
}

impl<T> LogIfErr for anyhow::Result<T>
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

impl<T> LogIfErr for windows::core::Result<T>
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

impl<T> LogIfErr for std::io::Result<T>
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

impl<T> LogIfErr for win_open::Result<T>
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
