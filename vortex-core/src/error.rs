//! Error types for Vortex

use thiserror::Error;

/// Vortex error types
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// `CGroup` operation failed
    #[error("CGroup error: {message}")]
    CGroup {
        /// Error message
        message: String,
    },

    /// Namespace operation failed
    #[error("Namespace error: {message}")]
    Namespace {
        /// Error message
        message: String,
    },

    /// Permission denied
    #[error("Permission denied: {operation}")]
    PermissionDenied {
        /// Operation that was denied
        operation: String,
    },

    /// Invalid configuration
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        /// Error message
        message: String,
    },

    /// System error from nix
    #[error("System error: {0}")]
    System(#[from] nix::Error),

    /// Channel send error
    #[error("Channel send error")]
    ChannelSend,

    /// Task join error
    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ChannelSend
    }
}

/// Result type alias for Vortex operations
pub type Result<T> = std::result::Result<T, Error>;
