//! Container lifecycle events with structured tracing

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, SystemTime};

use crate::{ContainerId, ResourceStats};

/// Events emitted during container lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContainerEvent {
    /// Container started
    Started {
        /// Container ID
        id: ContainerId,
        /// Timestamp
        #[serde(with = "systemtime_serde")]
        timestamp: SystemTime,
    },

    /// CPU was throttled
    CpuThrottled {
        /// Container ID
        id: ContainerId,
        /// Duration of throttling
        #[serde(with = "duration_serde")]
        duration: Duration,
        /// Timestamp
        #[serde(with = "systemtime_serde")]
        timestamp: SystemTime,
    },

    /// Memory pressure detected
    MemoryPressure {
        /// Container ID
        id: ContainerId,
        /// Current memory usage
        current: u64,
        /// Memory limit
        limit: u64,
        /// Percentage of limit
        percentage: f64,
        /// Timestamp
        #[serde(with = "systemtime_serde")]
        timestamp: SystemTime,
    },

    /// Container exiting
    Exiting {
        /// Container ID
        id: ContainerId,
        /// Exit code
        exit_code: i32,
        /// Timestamp
        #[serde(with = "systemtime_serde")]
        timestamp: SystemTime,
    },

    /// Stats update
    StatsUpdate {
        /// Container ID
        id: ContainerId,
        /// Resource statistics
        stats: ResourceStats,
        /// Timestamp
        #[serde(with = "systemtime_serde")]
        timestamp: SystemTime,
    },

    /// Error occurred
    Error {
        /// Container ID
        id: ContainerId,
        /// Error message
        message: String,
        /// Timestamp
        #[serde(with = "systemtime_serde")]
        timestamp: SystemTime,
    },
}

impl ContainerEvent {
    /// Get the container ID from any event
    #[must_use]
    pub const fn container_id(&self) -> &ContainerId {
        match self {
            Self::Started { id, .. }
            | Self::CpuThrottled { id, .. }
            | Self::MemoryPressure { id, .. }
            | Self::Exiting { id, .. }
            | Self::StatsUpdate { id, .. }
            | Self::Error { id, .. } => id,
        }
    }

    /// Get the timestamp from any event
    #[must_use]
    pub const fn timestamp(&self) -> SystemTime {
        match self {
            Self::Started { timestamp, .. }
            | Self::CpuThrottled { timestamp, .. }
            | Self::MemoryPressure { timestamp, .. }
            | Self::Exiting { timestamp, .. }
            | Self::StatsUpdate { timestamp, .. }
            | Self::Error { timestamp, .. } => *timestamp,
        }
    }

    /// Check if this is a critical event
    #[must_use]
    pub const fn is_critical(&self) -> bool {
        matches!(self, Self::MemoryPressure { .. } | Self::Error { .. })
    }

    /// Emit structured tracing event
    pub fn emit_trace(&self) {
        match self {
            Self::Started { id, .. } => {
                tracing::info!(
                    container_id = %id,
                    event = "started",
                    "Container started"
                );
            }
            Self::CpuThrottled { id, duration, .. } => {
                tracing::warn!(
                    container_id = %id,
                    duration_ms = duration.as_millis(),
                    event = "cpu_throttled",
                    "CPU throttled"
                );
            }
            Self::MemoryPressure {
                id,
                current,
                limit,
                percentage,
                ..
            } => {
                tracing::warn!(
                    container_id = %id,
                    current_mb = current / (1024 * 1024),
                    limit_mb = limit / (1024 * 1024),
                    percentage,
                    event = "memory_pressure",
                    "Memory pressure"
                );
            }
            Self::Exiting { id, exit_code, .. } => {
                tracing::info!(
                    container_id = %id,
                    exit_code,
                    event = "exiting",
                    "Container exiting"
                );
            }
            Self::StatsUpdate { id, .. } => {
                tracing::trace!(
                    container_id = %id,
                    event = "stats_update",
                    "Stats update"
                );
            }
            Self::Error { id, message, .. } => {
                tracing::error!(
                    container_id = %id,
                    message = %message,
                    event = "error",
                    "Container error"
                );
            }
        }
    }
}

impl fmt::Display for ContainerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Started { id, .. } => write!(f, "Container {} started", id),
            Self::CpuThrottled { id, duration, .. } => {
                write!(f, "Container {} CPU throttled for {:?}", id, duration)
            }
            Self::MemoryPressure { id, percentage, .. } => {
                write!(f, "Container {} memory at {:.1}%", id, percentage)
            }
            Self::Exiting { id, exit_code, .. } => {
                write!(f, "Container {} exiting with code {}", id, exit_code)
            }
            Self::StatsUpdate { id, .. } => {
                write!(f, "Container {} stats update", id)
            }
            Self::Error { id, message, .. } => {
                write!(f, "Container {} error: {}", id, message)
            }
        }
    }
}

// Custom Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    #[allow(clippy::cast_possible_truncation)]
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

// Custom SystemTime serialization
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let since_epoch = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(since_epoch.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_container_id() {
        let id = ContainerId::new("test").unwrap();
        let event = ContainerEvent::Started {
            id: id.clone(),
            timestamp: SystemTime::now(),
        };

        assert_eq!(event.container_id(), &id);
    }

    #[test]
    fn test_event_critical() {
        let id = ContainerId::new("test").unwrap();

        let event = ContainerEvent::Error {
            id: id.clone(),
            message: "test".to_string(),
            timestamp: SystemTime::now(),
        };
        assert!(event.is_critical());

        let event = ContainerEvent::Started {
            id,
            timestamp: SystemTime::now(),
        };
        assert!(!event.is_critical());
    }

    #[test]
    fn test_event_serde() {
        let id = ContainerId::new("test").unwrap();
        let event = ContainerEvent::Started {
            id,
            timestamp: SystemTime::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ContainerEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.container_id(), deserialized.container_id());
    }
}
