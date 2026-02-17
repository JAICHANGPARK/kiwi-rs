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

#[cfg(test)]
mod error_tests {
    use super::KiwiError;
    use std::ffi::CString;

    #[test]
    fn display_messages_are_human_readable() {
        assert_eq!(
            KiwiError::LibraryLoad("missing".to_string()).to_string(),
            "failed to load library: missing"
        );
        assert_eq!(
            KiwiError::SymbolLoad("kiwi_version".to_string()).to_string(),
            "failed to load symbol: kiwi_version"
        );
        assert_eq!(
            KiwiError::InvalidArgument("bad arg".to_string()).to_string(),
            "invalid argument: bad arg"
        );
        assert_eq!(
            KiwiError::Bootstrap("network".to_string()).to_string(),
            "bootstrap error: network"
        );
        assert_eq!(
            KiwiError::Api("ffi failed".to_string()).to_string(),
            "kiwi api error: ffi failed"
        );
    }

    #[test]
    fn nul_error_converts_to_kiwi_error() {
        let nul = CString::new("ab\0cd").expect_err("expected interior NUL");
        let error: KiwiError = nul.into();
        assert!(matches!(error, KiwiError::NulByte(_)));
        assert!(error.to_string().starts_with("string contains NUL byte:"));
    }
}
