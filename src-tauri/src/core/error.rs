//! Error types for the application

use thiserror::Error;

/// Application-wide error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Power monitoring error: {0}")]
    PowerMonitor(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Hardware not supported: {0}")]
    HardwareNotSupported(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// Result type alias using our Error
pub type Result<T> = std::result::Result<T, Error>;
