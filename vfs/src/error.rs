use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Interrupted,
    WriteZero,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: &'static str,
}

enum Message {
    Static(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Interrupted => write!(f, ""),
        }
    }
}

impl avagarden::error::Error for Error {}

impl Error {
    pub const fn new_const(kind: ErrorKind, message: &'static str) -> Error {
        Error { kind, message }
    }
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error {
            kind: value,
            message: "",
        }
    }
}
