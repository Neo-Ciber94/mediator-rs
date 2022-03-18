use std::fmt;

/// Error type for mediator
#[derive(PartialEq, Eq)]
pub struct Error {
    repr: ErrorRepr,
}

/// Error kind.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Handler not found.
    NotFound,
    /// An unknown error.
    Unknown,
}

impl ErrorKind {
    /// Returns the description of the error kind.
    pub fn as_str(&self) -> &str {
        match *self {
            ErrorKind::NotFound => "handler not found",
            ErrorKind::Unknown => "unknown error",
        }
    }
}

#[derive(PartialEq, Eq)]
enum ErrorRepr {
    /// An error with a kind.
    Kind(ErrorKind),
    /// An error with a description.
    WithDescription(ErrorKind, String),
}

impl Error {
    pub fn new<S: Into<String>>(kind: ErrorKind, description: S) -> Error {
        Error {
            repr: ErrorRepr::WithDescription(kind, description.into()),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            repr: ErrorRepr::Kind(kind),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.repr {
            ErrorRepr::Kind(ref kind) => {
                write!(f, "{}", kind.as_str())
            }
            ErrorRepr::WithDescription(ref kind, ref description) => match *kind {
                ErrorKind::Unknown => {
                    write!(f, "{}", description)
                }
                _ => {
                    write!(f, "{}: {}", kind.as_str(), description)
                }
            },
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {}
