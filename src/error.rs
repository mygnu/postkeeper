use std::fmt;
/// Custom Error type to represent errors for this crate
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    kind: Kind,
    msg: Option<String>,
}

/// This enumeration is the main information about an error. Every Error type
/// must be constructed with at least this information, that allows to differentiate
/// errors between each other with minimal information. The information of the
/// Kind is always related to an entity, but the entity can or cannot be known
/// at the moment of the error, therefore is it reported as an Optional field in
/// the main Error type struct.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    ConfigError,
    Internal,
}

impl Error {
    /// Creates an Error instance with the given error type and a message.
    pub fn with_msg(kind: Kind, msg: impl fmt::Display) -> Self {
        Self {
            kind,
            msg: Some(msg.to_string()),
        }
    }

    /// Creates an Error instance with the given error type and a message.
    pub fn config_err(msg: impl fmt::Display) -> Self {
        Self {
            kind: Kind::ConfigError,
            msg: Some(msg.to_string()),
        }
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(self)
    }
}

impl fmt::Display for Error {
    /// Formats service error for logging purposes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self.msg.as_ref() {
            Some(msg) => msg,
            None => "",
        };
        write!(f, "{:?}: {}", self.kind, msg)
    }
}

impl Default for Error {
    fn default() -> Self {
        Self {
            kind: Kind::Internal,
            msg: None,
        }
    }
}

/// PostKeeper Result type with custom error
pub type Result<T> = std::result::Result<T, Error>;

use ini::ini::Error as IniError;
use std::io::ErrorKind;

impl From<IniError> for Error {
    fn from(e: IniError) -> Self {
        match e {
            IniError::Io(ioe) => match ioe.kind() {
                ErrorKind::NotFound => Error::config_err("Config file not found"),
                ErrorKind::PermissionDenied => Error::config_err("Could not Read Config file"),
                _ => Error::config_err(ioe),
            },
            IniError::Parse(e) => Error::config_err(e),
        }
    }
}
