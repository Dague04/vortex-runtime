//! Core type definitions with strong typing and validation

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::{Error, Result};

/// Container identifier with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(try_from = "String", into = "String")]
pub struct ContainerId(String);

impl ContainerId {
    /// Maximum length for container IDs
    pub const MAX_LENGTH: usize = 64;

    /// Create a new `ContainerId` with validation
    ///
    /// # Errors
    /// Returns error if ID is invalid (empty, too long, or contains invalid characters)
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        Self::validate(&id)?;
        Ok(Self(id))
    }

    /// Validate a container ID
    fn validate(id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(Error::InvalidConfig {
                message: "Container ID cannot be empty".to_string(),
            });
        }

        if id.len() > Self::MAX_LENGTH {
            return Err(Error::InvalidConfig {
                message: format!("Container ID too long (max {} chars)", Self::MAX_LENGTH),
            });
        }

        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::InvalidConfig {
                message: "Container ID can only contain alphanumeric, dash, and underscore"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Get the container ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ContainerId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

impl TryFrom<String> for ContainerId {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Self::new(s)
    }
}

impl From<ContainerId> for String {
    fn from(id: ContainerId) -> Self {
        id.0
    }
}

/// Process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct ProcessId(i32);

impl ProcessId {
    /// Create from raw PID
    #[must_use]
    pub const fn from_raw(pid: i32) -> Self {
        Self(pid)
    }

    /// Get the current process ID
    #[must_use]
    pub fn current() -> Self {
        #[allow(clippy::cast_possible_wrap)]
        Self(std::process::id() as i32)
    }

    /// Convert to `nix::unistd::Pid`
    #[must_use]
    pub const fn as_nix_pid(self) -> nix::unistd::Pid {
        nix::unistd::Pid::from_raw(self.0)
    }

    /// Get raw PID value
    #[must_use]
    pub const fn as_raw(self) -> i32 {
        self.0
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<nix::unistd::Pid> for ProcessId {
    fn from(pid: nix::unistd::Pid) -> Self {
        Self(pid.as_raw())
    }
}

impl From<ProcessId> for nix::unistd::Pid {
    fn from(pid: ProcessId) -> Self {
        nix::unistd::Pid::from_raw(pid.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_id_validation() {
        assert!(ContainerId::new("valid-id_123").is_ok());
        assert!(ContainerId::new("").is_err());
        assert!(ContainerId::new("a".repeat(65)).is_err());
        assert!(ContainerId::new("invalid id").is_err());
        assert!(ContainerId::new("invalid/id").is_err());
    }

    #[test]
    fn test_container_id_serde() {
        let id = ContainerId::new("test-123").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ContainerId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_process_id() {
        let pid = ProcessId::from_raw(123);
        assert_eq!(pid.as_raw(), 123);

        let nix_pid = pid.as_nix_pid();
        assert_eq!(nix_pid.as_raw(), 123);
    }
}
