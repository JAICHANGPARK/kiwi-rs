use std::fmt;

/// Error type returned by kiwi-rs public APIs.
#[derive(Debug)]
pub enum KiwiError {
    /// Dynamic library could not be loaded.
    LibraryLoad(String),
    /// Required symbol could not be resolved from the library.
    SymbolLoad(String),
    /// Rust string contained an interior `NUL` byte for C interop.
    NulByte(std::ffi::NulError),
    /// User-provided arguments were invalid.
    InvalidArgument(String),
    /// Automatic asset bootstrap/download failed.
    Bootstrap(String),
    /// Error reported by the Kiwi C API.
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

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, KiwiError>;
