use core::fmt;

use alloc::string::{String, ToString};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Interrupted,
    WriteZero,
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionAborted,
    ConnectionReset,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidFilename,
    InvalidData,
    TimedOut,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        // tidy-alphabetical-start
        match *self {
            AddrInUse => "address in use",
            AddrNotAvailable => "address not available",
            AlreadyExists => "entity already exists",
            // ArgumentListTooLong => "argument list too long",
            BrokenPipe => "broken pipe",
            ConnectionAborted => "connection aborted",
            ConnectionRefused => "connection refused",
            ConnectionReset => "connection reset",
            // CrossesDevices => "cross-device link or rename",
            // Deadlock => "deadlock",
            // DirectoryNotEmpty => "directory not empty",
            // ExecutableFileBusy => "executable file busy",
            // FileTooLarge => "file too large",
            // FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            // FilesystemQuotaExceeded => "filesystem quota exceeded",
            // HostUnreachable => "host unreachable",
            Interrupted => "operation interrupted",
            InvalidData => "invalid data",
            InvalidFilename => "invalid filename",
            InvalidInput => "invalid input parameter",
            // IsADirectory => "is a directory",
            // NetworkDown => "network down",
            // NetworkUnreachable => "network unreachable",
            // NotADirectory => "not a directory",
            NotConnected => "not connected",
            NotFound => "entity not found",
            // NotSeekable => "seek on unseekable file",
            Other => "other error",
            OutOfMemory => "out of memory",
            PermissionDenied => "permission denied",
            // ReadOnlyFilesystem => "read-only filesystem or storage medium",
            // ResourceBusy => "resource busy",
            // StaleNetworkFileHandle => "stale network file handle",
            // StorageFull => "no storage space",
            TimedOut => "timed out",
            // TooManyLinks => "too many links",
            // Uncategorized => "uncategorized error",
            UnexpectedEof => "unexpected end of file",
            Unsupported => "unsupported",
            WouldBlock => "operation would block",
            WriteZero => "write zero",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: Option<Message>,
}

#[derive(Debug)]
enum Message {
    Static(&'static str),
    Owned(String),
}

impl Message {
    fn as_str(&self) -> &str {
        match self {
            Message::Owned(s) => s,
            Message::Static(s) => &s,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(msg) = &self.message {
            write!(f, ": {}", msg.as_str())?;
        }

        Ok(())
    }
}

impl avagarden::error::Error for Error {}

impl Error {
    pub const fn new_const(kind: ErrorKind, message: &'static str) -> Error {
        Error {
            kind,
            message: Some(Message::Static(message)),
        }
    }

    pub fn new(kind: ErrorKind, message: impl ToString) -> Error {
        Error {
            kind,
            message: Some(Message::Owned(message.to_string())),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error {
            kind: value,
            message: None,
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        let kind = match value.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
            std::io::ErrorKind::ConnectionRefused => ErrorKind::ConnectionRefused,
            std::io::ErrorKind::ConnectionReset => ErrorKind::ConnectionReset,
            std::io::ErrorKind::ConnectionAborted => ErrorKind::ConnectionAborted,
            std::io::ErrorKind::NotConnected => ErrorKind::NotConnected,
            std::io::ErrorKind::AddrInUse => ErrorKind::AddrInUse,
            std::io::ErrorKind::AddrNotAvailable => ErrorKind::AddrNotAvailable,
            std::io::ErrorKind::BrokenPipe => ErrorKind::BrokenPipe,
            std::io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
            std::io::ErrorKind::WouldBlock => ErrorKind::WouldBlock,
            std::io::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
            std::io::ErrorKind::InvalidData => ErrorKind::InvalidData,
            std::io::ErrorKind::TimedOut => ErrorKind::TimedOut,
            std::io::ErrorKind::WriteZero => ErrorKind::WriteZero,
            std::io::ErrorKind::Interrupted => ErrorKind::Interrupted,
            std::io::ErrorKind::Unsupported => ErrorKind::Unsupported,
            std::io::ErrorKind::UnexpectedEof => ErrorKind::UnexpectedEof,
            std::io::ErrorKind::OutOfMemory => ErrorKind::OutOfMemory,
            std::io::ErrorKind::Other => ErrorKind::Other,
            e => {
                return Error {
                    kind: ErrorKind::Other,
                    message: Some(Message::Owned(e.to_string())),
                }
            }
        };

        Error {
            kind,
            message: None,
        }
    }
}
