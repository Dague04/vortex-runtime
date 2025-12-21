//! Resource limit types with unit awareness.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};
use std::time::Duration;

/// Memory size with automatic unit conversion
///
/// # Example
///
/// ```rust
/// use vortex_core::MemorySize;
///
/// let mem = MemorySize::from_mb(512);
/// assert_eq!(mem.as_bytes(), 536_870_912);
/// println!("Memory: {}", mem); // "Memory: 512.00 MB"
///
/// let total = mem + MemorySize::from_mb(256);
/// assert_eq!(total.as_mb(), 768.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derive(Default)]
pub struct MemorySize(u64);

impl MemorySize {
    /// Create from bytes
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Create from kilobytes
    pub const fn from_kb(kb: u64) -> Self {
        Self(kb * 1024)
    }

    /// Create from megabytes
    pub const fn from_mb(mb: u64) -> Self {
        Self(mb * 1024 * 1024)
    }

    /// Create from gigabytes
    pub const fn from_gb(gb: u64) -> Self {
        Self(gb * 1024 * 1024 * 1024)
    }

    /// Get size in bytes
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    /// Get size in kilobytes
    pub fn as_kb(self) -> f64 {
        self.0 as f64 / 1024.0
    }

    /// Get size in megabytes
    pub fn as_mb(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0)
    }

    /// Get size in gigabytes
    pub fn as_gb(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

impl fmt::Display for MemorySize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1024 * 1024 * 1024 {
            write!(f, "{:.2} GB", self.as_gb())
        } else if self.0 >= 1024 * 1024 {
            write!(f, "{:.2} MB", self.as_mb())
        } else if self.0 >= 1024 {
            write!(f, "{:.2} KB", self.as_kb())
        } else {
            write!(f, "{} bytes", self.0)
        }
    }
}

impl Add for MemorySize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for MemorySize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

/// CPU cores (supports fractional cores)
///
/// # Example
///
/// ```rust
/// use vortex_core::CpuCores;
///
/// let cores = CpuCores::new(2.5);
/// println!("CPU: {}", cores); // "CPU: 2.50 cores"
///
/// let quota = cores.to_quota();
/// assert_eq!(quota.as_micros(), 250_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CpuCores(f64);

impl CpuCores {
    /// Create CPU cores specification
    pub const fn new(cores: f64) -> Self {
        Self(cores)
    }

    /// Get as f64
    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// Convert to CGroup v2 quota (microseconds per 100ms period)
    pub fn to_quota(self) -> Duration {
        Duration::from_micros((self.0 * 100_000.0) as u64)
    }

    /// Get the standard CGroup period (100ms)
    pub const fn period() -> Duration {
        Duration::from_micros(100_000)
    }
}

impl fmt::Display for CpuCores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} cores", self.0)
    }
}

/// Network bandwidth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bandwidth(u64);

impl Bandwidth {
    /// Create from bytes per second
    pub const fn from_bps(bps: u64) -> Self {
        Self(bps)
    }

    /// Create from kilobytes per second
    pub const fn from_kbps(kbps: u64) -> Self {
        Self(kbps * 1024)
    }

    /// Create from megabytes per second
    pub const fn from_mbps(mbps: u64) -> Self {
        Self(mbps * 1024 * 1024)
    }

    /// Get bytes per second
    pub const fn as_bps(self) -> u64 {
        self.0
    }

    /// Get megabytes per second
    pub fn as_mbps(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0)
    }
}

impl fmt::Display for Bandwidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1024 * 1024 {
            write!(f, "{:.2} MB/s", self.as_mbps())
        } else {
            write!(f, "{} bytes/s", self.0)
        }
    }
}

/// CPU limit specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpuLimit {
    /// Number of CPU cores
    pub cores: CpuCores,
}

impl CpuLimit {
    /// Create a new CPU limit
    pub fn new(cores: CpuCores) -> Self {
        Self { cores }
    }
}

/// Memory limit specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryLimit {
    /// Memory limit
    pub memory: MemorySize,
    /// Optional swap limit
    pub swap: Option<MemorySize>,
}

impl MemoryLimit {
    /// Create a new memory limit
    pub fn new(memory: MemorySize) -> Self {
        Self { memory, swap: None }
    }

    /// Add swap limit (builder pattern)
    pub fn with_swap(mut self, swap: MemorySize) -> Self {
        self.swap = Some(swap);
        self
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceStats {
    /// CPU time used
    pub cpu_usage: Duration,
    /// CPU time throttled
    pub cpu_throttled: Duration,
    /// Current memory usage
    pub memory_current: MemorySize,
    /// Peak memory usage
    pub memory_peak: MemorySize,
    /// Bytes read from disk
    pub io_read_bytes: u64,
    /// Bytes written to disk
    pub io_write_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_size_conversions() {
        let mem = MemorySize::from_mb(512);
        assert_eq!(mem.as_bytes(), 536_870_912);
        assert_eq!(mem.as_mb(), 512.0);
    }

    #[test]
    fn test_memory_size_arithmetic() {
        let a = MemorySize::from_mb(256);
        let b = MemorySize::from_mb(256);
        let sum = a + b;
        assert_eq!(sum.as_mb(), 512.0);
    }

    #[test]
    fn test_memory_size_display() {
        assert_eq!(MemorySize::from_bytes(512).to_string(), "512 bytes");
        assert_eq!(MemorySize::from_kb(1).to_string(), "1.00 KB");
        assert_eq!(MemorySize::from_mb(1).to_string(), "1.00 MB");
        assert_eq!(MemorySize::from_gb(1).to_string(), "1.00 GB");
    }

    #[test]
    fn test_cpu_quota_conversion() {
        let cores = CpuCores::new(2.0);
        let quota = cores.to_quota();
        assert_eq!(quota.as_micros(), 200_000);
    }

    #[test]
    fn test_cpu_display() {
        let cores = CpuCores::new(2.5);
        assert_eq!(cores.to_string(), "2.50 cores");
    }

    #[test]
    fn test_memory_limit_builder() {
        let limit = MemoryLimit::new(MemorySize::from_mb(512))
            .with_swap(MemorySize::from_mb(1024));

        assert_eq!(limit.memory.as_mb(), 512.0);
        assert_eq!(limit.swap.unwrap().as_mb(), 1024.0);
    }
}