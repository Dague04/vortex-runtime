//! Resource value objects with compile-time unit safety

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};
use std::time::Duration;

/// Memory size value object with compile-time unit safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[repr(transparent)]
#[serde(transparent)]
pub struct MemorySize(u64); // bytes

impl MemorySize {
    /// Create from bytes
    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Create from kilobytes
    #[must_use]
    pub const fn from_kb(kb: u64) -> Self {
        Self(kb.saturating_mul(1024))
    }

    /// Create from megabytes
    #[must_use]
    pub const fn from_mb(mb: u64) -> Self {
        Self(mb.saturating_mul(1024).saturating_mul(1024))
    }

    /// Create from gigabytes
    #[must_use]
    pub const fn from_gb(gb: u64) -> Self {
        Self(
            gb.saturating_mul(1024)
                .saturating_mul(1024)
                .saturating_mul(1024),
        )
    }

    /// Get value in bytes
    #[must_use]
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    /// Get value in kilobytes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_kb(self) -> f64 {
        self.0 as f64 / 1024.0
    }

    /// Get value in megabytes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_mb(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0)
    }

    /// Get value in gigabytes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_gb(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

impl Add for MemorySize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for MemorySize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl fmt::Display for MemorySize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const GB: u64 = 1024 * 1024 * 1024;
        const MB: u64 = 1024 * 1024;
        const KB: u64 = 1024;

        if self.0 >= GB {
            write!(f, "{:.2} GB", self.as_gb())
        } else if self.0 >= MB {
            write!(f, "{:.2} MB", self.as_mb())
        } else if self.0 >= KB {
            write!(f, "{:.2} KB", self.as_kb())
        } else {
            write!(f, "{} bytes", self.0)
        }
    }
}

/// CPU cores value object
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct CpuCores(f64);

impl CpuCores {
    /// Create new CPU cores value
    #[must_use]
    pub const fn new(cores: f64) -> Self {
        Self(cores)
    }

    /// Get value as f64
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }

    /// Convert to `CGroup` quota/period format
    ///
    /// Returns (quota, period) in microseconds
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn to_quota(self) -> (i64, i64) {
        const PERIOD: i64 = 100_000; // 100ms = 100,000 microseconds
        let quota = (self.0 * PERIOD as f64) as i64;
        (quota, PERIOD)
    }
}

/// CPU resource limit
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpuLimit {
    /// Number of CPU cores
    pub cores: CpuCores,
}

impl CpuLimit {
    /// Create new CPU limit
    #[must_use]
    pub const fn new(cores: CpuCores) -> Self {
        Self { cores }
    }
}

/// Memory resource limit
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryLimit {
    /// Memory limit
    pub limit: MemorySize,
    /// Optional swap limit
    pub swap: Option<MemorySize>,
}

impl MemoryLimit {
    /// Create new memory limit without swap
    #[must_use]
    pub const fn new(limit: MemorySize) -> Self {
        Self { limit, swap: None }
    }

    /// Create new memory limit with swap
    #[must_use]
    pub const fn with_swap(limit: MemorySize, swap: MemorySize) -> Self {
        Self {
            limit,
            swap: Some(swap),
        }
    }
}

/// Resource usage statistics snapshot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceStats {
    /// Total CPU time used
    #[serde(with = "duration_serde")]
    pub cpu_usage: Duration,

    /// Time spent throttled (hit CPU limit)
    #[serde(with = "duration_serde")]
    pub cpu_throttled: Duration,

    /// Current memory usage
    pub memory_current: MemorySize,

    /// Peak memory usage
    pub memory_peak: MemorySize,

    /// Current swap usage
    pub swap_current: MemorySize,

    /// Peak swap usage
    pub swap_peak: MemorySize,

    /// Total bytes read from disk
    pub io_read_bytes: u64,

    /// Total bytes written to disk
    pub io_write_bytes: u64,
}

// Custom Duration serialization (serde_json doesn't handle Duration well)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_size_conversions() {
        let size = MemorySize::from_mb(512);
        assert_eq!(size.as_bytes(), 536_870_912);
        assert_eq!(size.as_mb(), 512.0);
    }

    #[test]
    fn memory_size_arithmetic() {
        let a = MemorySize::from_mb(256);
        let b = MemorySize::from_mb(256);
        let sum = a + b;
        assert_eq!(sum.as_mb(), 512.0);
    }

    #[test]
    fn memory_size_display() {
        assert_eq!(format!("{}", MemorySize::from_gb(2)), "2.00 GB");
        assert_eq!(format!("{}", MemorySize::from_mb(512)), "512.00 MB");
        assert_eq!(format!("{}", MemorySize::from_bytes(100)), "100 bytes");
    }

    #[test]
    fn cpu_quota_conversion() {
        let cores = CpuCores::new(1.0);
        let (quota, period) = cores.to_quota();
        assert_eq!(quota, 100_000);
        assert_eq!(period, 100_000);

        let cores = CpuCores::new(0.5);
        let (quota, period) = cores.to_quota();
        assert_eq!(quota, 50_000);
        assert_eq!(period, 100_000);
    }

    #[test]
    fn resource_stats_serde() {
        let stats = ResourceStats {
            cpu_usage: Duration::from_secs(10),
            cpu_throttled: Duration::from_millis(500),
            memory_current: MemorySize::from_mb(100),
            memory_peak: MemorySize::from_mb(150),
            ..Default::default()
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: ResourceStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.cpu_usage, deserialized.cpu_usage);
    }
}
