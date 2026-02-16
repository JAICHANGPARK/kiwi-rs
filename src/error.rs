use std::fmt;

#[derive(Debug)]
pub enum KiwiError {
    LibraryLoad(String),
    SymbolLoad(String),
    NulByte(std::ffi::NulError),
    InvalidArgument(String),
    Bootstrap(String),
    Api(String),
}

impl fmt::Display for KiwiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KiwiError::LibraryLoad(message) => write!(f, "failed to load library: {message}"),
            KiwiError::SymbolLoad(message) => write!(f, "failed to load symbol: {message}"),
            KiwiError::NulByte(error) => write!(f, "string contains NUL byte: {error}"),
            KiwiError::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            KiwiError::Bootstrap(message) => write!(f, "bootstrap error: {message}"),
            KiwiError::Api(message) => write!(f, "kiwi api error: {message}"),
        }
    }
}

impl std::error::Error for KiwiError {}

impl From<std::ffi::NulError> for KiwiError {
    fn from(value: std::ffi::NulError) -> Self {
        KiwiError::NulByte(value)
    }
}

pub type Result<T> = std::result::Result<T, KiwiError>;
