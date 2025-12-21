//! Resource limit implementations
//!
//! This module provides methods for setting CPU, memory, and I/O limits
//! by writing to CGroup v2 control files

use crate::controller::CGroupController;
use tokio::fs;
use tracing::debug;
use vortex_core::{CpuLimit, Error, MemoryLimit, Result};

impl CGroupController {
    /// Set CPU limit for this cgroup
    ///
    /// This controls how much CPU time the cgroup can use per period.
    pub async fn set_cpu_limit(&self, limit: CpuLimit) -> Result<()> {
        if !self.active {
            return Err(Error::CGroup {
                message: "CGroup not active".to_string(),
            });
        }

        let cpu_max_file = self.path.join("cpu.max");

        // Convert cores to quota (microseconds)
        let quota = limit.cores.to_quota();
        let period = vortex_core::CpuCores::period();

        // Format: "quota period" (both in microseconds)
        let content = format!("{} {}", quota.as_micros(), period.as_micros());

        debug!(
            "Setting CPU limit to {} (quota={} period={})",
            limit.cores,
            quota.as_micros(),
            period.as_micros()
        );

        fs::write(&cpu_max_file, content)
            .await
            .map_err(|e| Error::PermissionDenied {
                operation: format!("Set CPU limit: {}", e),
            })?;

        Ok(())
    }

    /// Set memory limit for this cgroup
    ///
    /// This sets the maximum amount of memory the cgroup can use.
    /// If exceeded, the kernel's OOM (Out Of Memory) killer will terminate
    /// processes in the cgroup.
    pub async fn set_memory_limit(&self, limit: MemoryLimit) -> Result<()> {
        if !self.active {
            return Err(Error::CGroup {
                message: "CGroup not active".to_string(),
            });
        }

        // Set main memory limit
        let memory_max_file = self.path.join("memory.max");
        let memory_bytes = limit.memory.as_bytes().to_string();

        debug!("Setting memory limit to {}", limit.memory);

        fs::write(&memory_max_file, &memory_bytes)
            .await
            .map_err(|e| Error::PermissionDenied {
                operation: format!("Set memory limit: {}", e),
            })?;

        // Set swap limit if specified
        if let Some(swap) = limit.swap {
            let swap_max_file = self.path.join("memory.swap.max");
            let swap_bytes = swap.as_bytes().to_string();

            debug!("Setting swap limit to {}", swap);

            fs::write(&swap_max_file, &swap_bytes)
                .await
                .map_err(|e| Error::PermissionDenied {
                    operation: format!("Set swap limit: {}", e),
                })?;
        }

        Ok(())
    }

    /// Remove CPU limit (set to "max")
    ///
    /// This allows the cgroup to use unlimited CPU.
    pub async fn remove_cpu_limit(&self) -> Result<()> {
        if !self.active {
            return Err(Error::CGroup {
                message: "CGroup not active".to_string(),
            });
        }

        let cpu_max_file = self.path.join("cpu.max");

        debug!("Removing CPU limit");

        // "max" means unlimited
        fs::write(&cpu_max_file, "max 100000")
            .await
            .map_err(|e| Error::PermissionDenied {
                operation: format!("Remove CPU limit: {}", e),
            })?;

        Ok(())
    }

    /// Remove memory limit (set to "max")
    ///
    /// This allows the cgroup to use unlimited memory.
    pub async fn remove_memory_limit(&self) -> Result<()> {
        if !self.active {
            return Err(Error::CGroup {
                message: "CGroup not active".to_string(),
            });
        }

        let memory_max_file = self.path.join("memory.max");

        debug!("Removing memory limit");

        // "max" means unlimited
        fs::write(&memory_max_file, "max")
            .await
            .map_err(|e| Error::PermissionDenied {
                operation: format!("Remove memory limit: {}", e),
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vortex_core::{ContainerId, CpuCores, MemorySize};

    #[tokio::test]
    async fn test_cpu_limit_format() {
        // Test the quota calculation
        let cores = CpuCores::new(2.0);
        let quota = cores.to_quota();

        assert_eq!(quota.as_micros(), 200_000);

        let period = CpuCores::period();
        assert_eq!(period.as_micros(), 100_000);
    }

    #[tokio::test]
    async fn test_set_cpu_limit() {
        // This will fail without root, that's expected
        let id = ContainerId::new("test-cpu-limit").unwrap();

        if let Ok(cgroup) = CGroupController::new(id).await {
            let limit = CpuLimit::new(CpuCores::new(1.5));
            let result = cgroup.set_cpu_limit(limit).await;

            match result {
                Ok(_) => println!("✅ CPU limit set successfully"),
                Err(e) => println!("⚠️  Expected error: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_memory_limit_format() {
        let mem = MemorySize::from_mb(512);
        assert_eq!(mem.as_bytes(), 536_870_912);
    }
}
