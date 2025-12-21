//! Error types for Vortex operations.
//!
//! This module uses `thiserror` for ergonomic error definitions.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// The main error type for vortex operations.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// CGroup operation failed
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
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Resource limit error
    #[error("Invalid resource limit: {resource} = {value}")]
    InvalidLimit {
        /// Resource type
        resource: String,
        /// Invalid value
        value: String,
    },

    /// Path does not exist
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    /// Container already exists
    #[error("Container already exists: {0}")]
    ContainerExists(String),

    /// Container not found
    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    /// System call failed
    #[error("System call '{syscall}' failed: {errno}")]
    Syscall {
        /// Syscall name
        syscall: String,
        /// Error number
        errno: i32,
    },
}

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;