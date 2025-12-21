//! Strongly-typed wrappers for system identifiers.
//!
//! Uses the newtype pattern for type safety.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Process ID - newtype wrapper around i32
///
/// # Example
///
/// ```rust
/// use vortex_core::ProcessId;
///
/// let pid = ProcessId::current();
/// println!("Current PID: {}", pid);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(i32);

impl ProcessId {
    /// Get the current process ID
    pub fn current() -> Self {
        Self(std::process::id() as i32)
    }

    /// Create from raw PID
    pub const fn from_raw(pid: i32) -> Self {
        Self(pid)
    }

    /// Get the raw PID value
    pub const fn as_raw(self) -> i32 {
        self.0
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Container ID - validated string wrapper
///
/// # Rules
///
/// - Must not be empty
/// - Can contain: alphanumeric, dash, underscore
/// - Cannot contain: spaces, special characters
///
/// # Example
///
/// ```rust
/// use vortex_core::ContainerId;
///
/// let id = ContainerId::new("my-container").unwrap();
/// assert_eq!(id.as_str(), "my-container");
///
/// // Invalid IDs fail
/// assert!(ContainerId::new("").is_err());
/// assert!(ContainerId::new("bad id").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerId(String);

impl ContainerId {
    /// Create a new container ID
    ///
    /// # Errors
    ///
    /// Returns error if ID is empty or contains invalid characters
    pub fn new(id: impl Into<String>) -> crate::Result<Self> {
        let id = id.into();

        if id.is_empty() {
            return Err(crate::Error::InvalidConfig(
                "Container ID cannot be empty".to_string(),
            ));
        }

        // Validate characters (alphanumeric, dash, underscore)
        if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(crate::Error::InvalidConfig(format!(
                "Invalid container ID '{}': must contain only alphanumeric, dash, or underscore",
                id
            )));
        }

        Ok(Self(id))
    }

    /// Get the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and get the inner String
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ContainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Convenient conversions
impl TryFrom<String> for ContainerId {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ContainerId {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_id() {
        let pid = ProcessId::from_raw(12345);
        assert_eq!(pid.as_raw(), 12345);
        assert_eq!(pid.to_string(), "12345");
    }

    #[test]
    fn test_container_id_valid() {
        let id = ContainerId::new("my-container").unwrap();
        assert_eq!(id.as_str(), "my-container");
    }

    #[test]
    fn test_container_id_empty() {
        let result = ContainerId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_container_id_invalid_chars() {
        assert!(ContainerId::new("bad id").is_err());
        assert!(ContainerId::new("bad@id").is_err());
        assert!(ContainerId::new("bad/id").is_err());
    }

    #[test]
    fn test_container_id_valid_chars() {
        assert!(ContainerId::new("valid-id_123").is_ok());
        assert!(ContainerId::new("container_1").is_ok());
        assert!(ContainerId::new("my-app-v2").is_ok());
    }
}