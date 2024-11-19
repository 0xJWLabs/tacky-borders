use serde::de;
use std::fmt;

use std::error;

/// Errors created from this crate.
#[derive(Debug, Clone)]
pub enum Error {
    /// A certain deserialization is impossible.
    ImpossibleDeserialization(&'static str),
    /// Raised when parsing errors happen during deserialization.
    Parse(&'static str, String),
    /// An arbitrary error message.
    Message(String),
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Error {
        Error::Message(msg.to_string())
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ImpossibleDeserialization(ty) => {
                write!(f, "cannot deserialize to non primitive type {}", ty)
            }
            Error::Parse(ref ty, ref msg) => write!(f, "cannot parse {}: {}", ty, msg),
            Error::Message(ref msg) => write!(f, "{}", msg.as_str()),
        }
    }
}
