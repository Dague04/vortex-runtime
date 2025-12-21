//! Resource statistics reading from CGroup v2
//!
//! This module provides methods to read resource usage statistics
//! from CGroup control files.

use std::time::Duration;
use tokio::fs;
use tracing::debug;
use vortex_core::{Error, MemorySize, ResourceStats, Result};

use crate::controller::CGroupController;

impl CGroupController {
    /// Get current resource statistics for this cgroup
    ///
    /// Reads from various cgroup stat files:
    /// - cpu.stat (CPU usage, throttling)
    /// - memory.current (current memory usage)
    /// - memory.peak (peak memory usage)
    /// - io.stat (I/O statistics)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use vortex_cgroup::CGroupController;
    /// # use vortex_core::ContainerId;
    /// # async fn example() -> vortex_core::Result<()> {
    /// let cgroup = CGroupController::new(ContainerId::new("my-app")?).await?;
    ///
    /// let stats = cgroup.stats().await?;
    /// println!("CPU usage: {:?}", stats.cpu_usage);
    /// println!("Memory: {}", stats.memory_current);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stats(&self) -> Result<ResourceStats> {
        if !self.active {
            return Err(Error::CGroup {
                message: "CGroup not active".to_string(),
            });
        }

        // Read CPU statistics
        let (cpu_usage, cpu_throttled) = self.read_cpu_stats().await?;

        // Read memory statistics
        let (memory_current, memory_peak) = self.read_memory_stats().await?;

        // Read I/O statistics
        let (io_read, io_write) = self.read_io_stats().await?;

        Ok(ResourceStats {
            cpu_usage,
            cpu_throttled,
            memory_current,
            memory_peak,
            io_read_bytes: io_read,
            io_write_bytes: io_write,
        })
    }

    /// Read CPU statistics from cpu.stat
    ///
    /// Format:
    /// ```text
    /// usage_usec 12345678
    /// user_usec 1234567
    /// system_usec 890123
    /// nr_periods 456
    /// nr_throttled 123
    /// throttled_usec 45678
    /// ```
    async fn read_cpu_stats(&self) -> Result<(Duration, Duration)> {
        let cpu_stat_file = self.path.join("cpu.stat");

        let content = fs::read_to_string(&cpu_stat_file)
            .await
            .map_err(|e| Error::CGroup {
                message: format!("Failed to read cpu.stat: {}", e),
            })?;

        let mut usage_usec = 0u64;
        let mut throttled_usec = 0u64;

        // Parse the file line by line
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 2 {
                continue;
            }

            match parts[0] {
                "usage_usec" => {
                    usage_usec = parts[1].parse().unwrap_or(0);
                }
                "throttled_usec" => {
                    throttled_usec = parts[1].parse().unwrap_or(0);
                }
                _ => {}
            }
        }

        debug!(
            "CPU stats: usage={}μs, throttled={}μs",
            usage_usec, throttled_usec
        );

        Ok((
            Duration::from_micros(usage_usec),
            Duration::from_micros(throttled_usec),
        ))
    }

    /// Read memory statistics
    ///
    /// Reads from:
    /// - memory.current (current usage)
    /// - memory.peak (peak usage since creation)
    async fn read_memory_stats(&self) -> Result<(MemorySize, MemorySize)> {
        // Read current memory usage
        let current_file = self.path.join("memory.current");
        let current_str = fs::read_to_string(&current_file)
            .await
            .map_err(|e| Error::CGroup {
                message: format!("Failed to read memory.current: {}", e),
            })?;

        let current_bytes: u64 = current_str.trim().parse().unwrap_or(0);
        let current = MemorySize::from_bytes(current_bytes);

        // Read peak memory usage
        let peak_file = self.path.join("memory.peak");
        let peak = match fs::read_to_string(&peak_file).await {
            Ok(peak_str) => {
                let peak_bytes: u64 = peak_str.trim().parse().unwrap_or(0);
                MemorySize::from_bytes(peak_bytes)
            }
            Err(_) => {
                // memory.peak might not exist on older kernels
                debug!("memory.peak not available, using current as peak");
                current
            }
        };

        debug!("Memory stats: current={}, peak={}", current, peak);

        Ok((current, peak))
    }

    /// Read I/O statistics from io.stat
    ///
    /// Format:
    /// ```text
    /// 8:0 rbytes=1234567 wbytes=890123 rios=456 wios=789
    /// 8:16 rbytes=111222 wbytes=333444 rios=55 wios=66
    /// ```
    ///
    /// We sum across all devices
    async fn read_io_stats(&self) -> Result<(u64, u64)> {
        let io_stat_file = self.path.join("io.stat");

        let content = match fs::read_to_string(&io_stat_file).await {
            Ok(c) => c,
            Err(_) => {
                // io.stat might not exist or be accessible
                debug!("io.stat not available");
                return Ok((0, 0));
            }
        };

        let mut total_read = 0u64;
        let mut total_write = 0u64;

        // Parse each device line
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            // Skip device identifier (e.g., "8:0")
            // Parse key=value pairs
            for part in &parts[1..] {
                if let Some((key, value)) = part.split_once('=') {
                    match key {
                        "rbytes" => {
                            if let Ok(bytes) = value.parse::<u64>() {
                                total_read += bytes;
                            }
                        }
                        "wbytes" => {
                            if let Ok(bytes) = value.parse::<u64>() {
                                total_write += bytes;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        debug!(
            "I/O stats: read={} bytes, write={} bytes",
            total_read, total_write
        );

        Ok((total_read, total_write))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vortex_core::ContainerId;

    #[tokio::test]
    async fn test_stats_inactive() {
        // Create but don't activate
        let mut controller = CGroupController {
            container_id: ContainerId::new("test").unwrap(),
            path: std::path::PathBuf::from("/tmp/test"),
            active: false,
        };

        let result = controller.stats().await;
        assert!(result.is_err());
    }
}
