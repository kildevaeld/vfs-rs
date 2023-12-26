use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Interrupted,
    WriteZero,
    NotFound,
    PermissionDenied,
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
        todo!()
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

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        let kind = match value.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
            std::io::ErrorKind::ConnectionRefused => todo!(),
            std::io::ErrorKind::ConnectionReset => todo!(),
            std::io::ErrorKind::ConnectionAborted => todo!(),
            std::io::ErrorKind::NotConnected => todo!(),
            std::io::ErrorKind::AddrInUse => todo!(),
            std::io::ErrorKind::AddrNotAvailable => todo!(),
            std::io::ErrorKind::BrokenPipe => todo!(),
            std::io::ErrorKind::AlreadyExists => todo!(),
            std::io::ErrorKind::WouldBlock => todo!(),
            std::io::ErrorKind::InvalidInput => todo!(),
            std::io::ErrorKind::InvalidData => todo!(),
            std::io::ErrorKind::TimedOut => todo!(),
            std::io::ErrorKind::WriteZero => todo!(),
            std::io::ErrorKind::Interrupted => todo!(),
            std::io::ErrorKind::Unsupported => todo!(),
            std::io::ErrorKind::UnexpectedEof => todo!(),
            std::io::ErrorKind::OutOfMemory => todo!(),
            std::io::ErrorKind::Other => todo!(),
            _ => todo!(),
        };

        Error { kind, message: "" }
    }
}
