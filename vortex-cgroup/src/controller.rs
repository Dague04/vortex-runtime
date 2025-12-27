//! CGroup v2 controller implementation

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::Mutex;
use vortex_core::{
    ContainerId, CpuLimit, Error, MemoryLimit, MemorySize, ProcessId, ResourceStats, Result,
};

use crate::backend::ResourceBackend;

/// CGroup v2 root path
const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// Vortex cgroup namespace
const VORTEX_NAMESPACE: &str = "vortex";

/// Delay for kernel cleanup operations (milliseconds)
const KERNEL_CLEANUP_DELAY_MS: u64 = 10;

/// Required CGroup controllers
const REQUIRED_CONTROLLERS: &[&str] = &["cpu", "memory", "io"];

/// CGroup v2 controller for resource management
pub struct CGroupController {
    container_id: ContainerId,
    path: PathBuf,
    active: bool,
}

/// Shared controller type for use with `Arc<Mutex<>>`
pub type SharedController = Arc<Mutex<CGroupController>>;

impl CGroupController {
    /// Create a new CGroup controller
    ///
    /// This will:
    /// 1. Create the cgroup directory hierarchy
    /// 2. Enable necessary controllers
    /// 3. Prepare for resource management
    ///
    /// # Errors
    /// Returns error if cgroup creation fails (e.g., permission denied)
    pub async fn new(container_id: ContainerId) -> Result<Self> {
        tracing::debug!(
            container_id = %container_id,
            "Creating CGroup controller"
        );

        let path = Path::new(CGROUP_ROOT)
            .join(VORTEX_NAMESPACE)
            .join(container_id.as_str());

        let mut controller = Self {
            container_id,
            path,
            active: true,
        };

        controller.create().await?;

        tracing::info!(
            container_id = %controller.container_id,
            path = %controller.path.display(),
            "CGroup controller created"
        );

        Ok(controller)
    }

    /// Create a shared (Arc<Mutex<>>) controller for concurrent access
    ///
    /// # Errors
    /// Returns error if controller creation fails
    pub async fn new_shared(container_id: ContainerId) -> Result<SharedController> {
        let controller = Self::new(container_id).await?;
        Ok(Arc::new(Mutex::new(controller)))
    }

    /// Get the container ID
    #[must_use]
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }

    /// Get the cgroup path
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if controller is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Create the cgroup directory hierarchy and enable controllers
    async fn create(&mut self) -> Result<()> {
        // Step 1: Create directory structure
        self.create_directory_hierarchy().await?;

        // Step 2: Enable controllers at each level
        self.enable_controllers_in_hierarchy().await?;

        Ok(())
    }

    /// Create the directory hierarchy for this cgroup
    async fn create_directory_hierarchy(&self) -> Result<()> {
        let root = Path::new(CGROUP_ROOT);
        let vortex_root = root.join(VORTEX_NAMESPACE);

        // Create vortex directory if it doesn't exist
        if !vortex_root.exists() {
            fs::create_dir_all(&vortex_root).await.map_err(|e| {
                tracing::error!(
                    path = %vortex_root.display(),
                    error = %e,
                    "Failed to create vortex directory"
                );
                Error::CGroup {
                    message: format!(
                        "Failed to create vortex directory: {}\nPath: {}",
                        e,
                        vortex_root.display()
                    ),
                }
            })?;

            tracing::info!(
                path = %vortex_root.display(),
                "Created vortex cgroup directory"
            );
        }

        // Create container directory
        fs::create_dir_all(&self.path).await.map_err(|e| {
            tracing::error!(
                path = %self.path.display(),
                error = %e,
                "Failed to create container directory"
            );
            Error::CGroup {
                message: format!(
                    "Failed to create container directory: {}\nPath: {}",
                    e,
                    self.path.display()
                ),
            }
        })?;

        tracing::debug!(
            path = %self.path.display(),
            "CGroup directory created"
        );

        Ok(())
    }

    /// Enable controllers at all levels in the hierarchy
    async fn enable_controllers_in_hierarchy(&self) -> Result<()> {
        let root = Path::new(CGROUP_ROOT);
        let vortex_root = root.join(VORTEX_NAMESPACE);

        // Enable at root level (best effort)
        self.enable_controllers_at(root).await;

        // Enable at vortex level (best effort)
        self.enable_controllers_at(&vortex_root).await;

        Ok(())
    }

    /// Enable controllers at a specific path
    ///
    /// This is best-effort and will not fail if controllers cannot be enabled
    /// (they might be managed by systemd or already enabled at a higher level)
    async fn enable_controllers_at(&self, path: &Path) {
        let controllers_file = path.join("cgroup.controllers");
        let control_file = path.join("cgroup.subtree_control");

        // Skip if control file doesn't exist
        if !control_file.exists() {
            tracing::trace!(
                path = %path.display(),
                "Subtree control file doesn't exist, skipping"
            );
            return;
        }

        // Read available controllers
        let available = match fs::read_to_string(&controllers_file).await {
            Ok(content) => content,
            Err(e) => {
                tracing::trace!(
                    path = %path.display(),
                    error = %e,
                    "Could not read available controllers"
                );
                return;
            }
        };

        // Read currently enabled controllers
        let enabled = fs::read_to_string(&control_file).await.unwrap_or_default();

        // Determine which controllers need to be enabled
        let to_enable: Vec<&str> = REQUIRED_CONTROLLERS
            .iter()
            .copied()
            .filter(|c| available.contains(c) && !enabled.contains(c))
            .collect();

        if to_enable.is_empty() {
            tracing::trace!(
                path = %path.display(),
                "All required controllers already enabled"
            );
            return;
        }

        // Try to enable each controller individually
        // This is more robust than enabling all at once
        for controller in &to_enable {
            let cmd = format!("+{}", controller);

            match fs::write(&control_file, &cmd).await {
                Ok(()) => {
                    tracing::debug!(
                        path = %path.display(),
                        controller = %controller,
                        "Enabled controller"
                    );
                }
                Err(e) => {
                    // Just log at debug level - this is expected in many cases
                    // (systemd management, already enabled at higher level, etc.)
                    tracing::debug!(
                        path = %path.display(),
                        controller = %controller,
                        error = %e,
                        "Could not enable controller (may be managed at higher level)"
                    );
                }
            }
        }
    }

    /// Cleanup the cgroup
    ///
    /// This will:
    /// 1. Move all processes back to root cgroup
    /// 2. Wait for kernel cleanup
    /// 3. Remove the cgroup directory
    ///
    /// # Errors
    /// Returns error if cleanup fails
    pub async fn cleanup(&mut self) -> Result<()> {
        if !self.active {
            tracing::debug!("CGroup already cleaned up");
            return Ok(());
        }

        tracing::debug!(
            container_id = %self.container_id,
            "Cleaning up cgroup"
        );

        // Move processes to root cgroup
        self.move_processes_to_root().await;

        // Small delay for kernel cleanup
        tokio::time::sleep(Duration::from_millis(KERNEL_CLEANUP_DELAY_MS)).await;

        // Remove directory
        self.remove_cgroup_directory().await;

        self.active = false;
        Ok(())
    }

    /// Move all processes in this cgroup back to the root cgroup
    async fn move_processes_to_root(&self) {
        let procs_file = self.path.join("cgroup.procs");
        let root_procs = Path::new(CGROUP_ROOT).join("cgroup.procs");

        match fs::read_to_string(&procs_file).await {
            Ok(pids_str) => {
                for line in pids_str.lines() {
                    if let Ok(pid) = line.trim().parse::<i32>() {
                        if let Err(e) = fs::write(&root_procs, pid.to_string()).await {
                            tracing::debug!(
                                pid = pid,
                                error = %e,
                                "Could not move process to root cgroup"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "Could not read process list"
                );
            }
        }
    }

    /// Remove the cgroup directory
    async fn remove_cgroup_directory(&self) {
        match fs::remove_dir(&self.path).await {
            Ok(()) => {
                tracing::info!(
                    container_id = %self.container_id,
                    path = %self.path.display(),
                    "CGroup removed"
                );
            }
            Err(e) => {
                tracing::warn!(
                    container_id = %self.container_id,
                    path = %self.path.display(),
                    error = %e,
                    "Failed to remove cgroup directory (may already be removed)"
                );
            }
        }
    }
}

/// Implement ResourceBackend trait for CGroupController
#[async_trait]
impl ResourceBackend for CGroupController {
    async fn set_cpu_limit(&self, limit: CpuLimit) -> Result<()> {
        let (quota, period) = limit.cores.to_quota();

        let cpu_max_file = self.path.join("cpu.max");
        let content = format!("{quota} {period}");

        fs::write(&cpu_max_file, content).await.map_err(|e| {
            tracing::error!(
                container_id = %self.container_id,
                error = %e,
                "Failed to set CPU limit"
            );
            Error::CGroup {
                message: format!("Failed to set CPU limit: {e}"),
            }
        })?;

        tracing::info!(
            container_id = %self.container_id,
            cores = limit.cores.as_f64(),
            quota,
            period,
            "Set CPU limit"
        );

        Ok(())
    }

    async fn set_memory_limit(&self, limit: MemoryLimit) -> Result<()> {
        // Set memory limit
        let memory_max_file = self.path.join("memory.max");
        let limit_bytes = limit.limit.as_bytes().to_string();

        fs::write(&memory_max_file, &limit_bytes)
            .await
            .map_err(|e| {
                tracing::error!(
                    container_id = %self.container_id,
                    error = %e,
                    "Failed to set memory limit"
                );
                Error::CGroup {
                    message: format!("Failed to set memory limit: {e}"),
                }
            })?;

        // Set swap limit if specified
        if let Some(swap) = limit.swap {
            let swap_max_file = self.path.join("memory.swap.max");
            let swap_bytes = swap.as_bytes().to_string();

            fs::write(&swap_max_file, &swap_bytes).await.map_err(|e| {
                tracing::error!(
                    container_id = %self.container_id,
                    error = %e,
                    "Failed to set swap limit"
                );
                Error::CGroup {
                    message: format!("Failed to set swap limit: {e}"),
                }
            })?;

            tracing::info!(
                container_id = %self.container_id,
                memory = %limit.limit,
                swap = %swap,
                "Set memory and swap limits"
            );
        } else {
            tracing::info!(
                container_id = %self.container_id,
                memory = %limit.limit,
                "Set memory limit"
            );
        }

        Ok(())
    }

    async fn add_process(&self, pid: ProcessId) -> Result<()> {
        let procs_file = self.path.join("cgroup.procs");
        let pid_str = pid.as_raw().to_string();

        fs::write(&procs_file, pid_str.as_bytes())
            .await
            .map_err(|e| {
                tracing::error!(
                    container_id = %self.container_id,
                    pid = pid.as_raw(),
                    error = %e,
                    "Failed to add process"
                );
                Error::CGroup {
                    message: format!("Failed to add process {pid}: {e}"),
                }
            })?;

        tracing::debug!(
            container_id = %self.container_id,
            pid = pid.as_raw(),
            "Added process to cgroup"
        );

        Ok(())
    }

    async fn stats(&self) -> Result<ResourceStats> {
        let cpu_stats = self.read_cpu_stats().await?;
        let memory_stats = self.read_memory_stats().await?;
        let io_stats = self.read_io_stats().await?;

        Ok(ResourceStats {
            cpu_usage: cpu_stats.0,
            cpu_throttled: cpu_stats.1,
            memory_current: memory_stats.0,
            memory_peak: memory_stats.1,
            swap_current: memory_stats.2,
            swap_peak: memory_stats.3,
            io_read_bytes: io_stats.0,
            io_write_bytes: io_stats.1,
        })
    }

    async fn cleanup(&self) -> Result<()> {
        tracing::warn!(
            "cleanup() called through trait interface - use controller.cleanup() directly for mutable access"
        );
        Ok(())
    }
}

impl CGroupController {
    async fn read_cpu_stats(&self) -> Result<(Duration, Duration)> {
        let cpu_stat_file = self.path.join("cpu.stat");

        let content = fs::read_to_string(&cpu_stat_file)
            .await
            .map_err(|e| Error::CGroup {
                message: format!("Failed to read cpu.stat: {e}"),
            })?;

        let mut usage_usec = 0u64;
        let mut throttled_usec = 0u64;

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

        Ok((
            Duration::from_micros(usage_usec),
            Duration::from_micros(throttled_usec),
        ))
    }

    async fn read_memory_stats(&self) -> Result<(MemorySize, MemorySize, MemorySize, MemorySize)> {
        let current = self.read_single_value("memory.current").await?;
        let peak = self.read_single_value("memory.peak").await?;
        let swap_current = self
            .read_single_value("memory.swap.current")
            .await
            .unwrap_or(0);
        let swap_peak = self
            .read_single_value("memory.swap.peak")
            .await
            .unwrap_or(0);

        Ok((
            MemorySize::from_bytes(current),
            MemorySize::from_bytes(peak),
            MemorySize::from_bytes(swap_current),
            MemorySize::from_bytes(swap_peak),
        ))
    }

    async fn read_io_stats(&self) -> Result<(u64, u64)> {
        let io_stat_file = self.path.join("io.stat");

        let content = fs::read_to_string(&io_stat_file).await.unwrap_or_default();

        let mut total_read = 0u64;
        let mut total_write = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            for part in &parts[1..] {
                if let Some((key, value)) = part.split_once('=') {
                    match key {
                        "rbytes" => {
                            total_read += value.parse::<u64>().unwrap_or(0);
                        }
                        "wbytes" => {
                            total_write += value.parse::<u64>().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok((total_read, total_write))
    }

    async fn read_single_value(&self, filename: &str) -> Result<u64> {
        let file = self.path.join(filename);
        let content = fs::read_to_string(&file).await.map_err(|e| Error::CGroup {
            message: format!("Failed to read {filename}: {e}"),
        })?;

        content.trim().parse().map_err(|e| Error::CGroup {
            message: format!("Failed to parse {filename} value: {e}"),
        })
    }
}

impl Drop for CGroupController {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        tracing::warn!(
            container_id = %self.container_id,
            "CGroup not explicitly cleaned up, using Drop fallback"
        );

        // Synchronous cleanup (best effort)
        let procs_file = self.path.join("cgroup.procs");
        if let Ok(pids_str) = std::fs::read_to_string(&procs_file) {
            let root_procs = Path::new(CGROUP_ROOT).join("cgroup.procs");
            for line in pids_str.lines() {
                if let Ok(pid) = line.trim().parse::<i32>() {
                    let _ = std::fs::write(&root_procs, pid.to_string());
                }
            }
        }

        std::thread::sleep(Duration::from_millis(KERNEL_CLEANUP_DELAY_MS));
        let _ = std::fs::remove_dir(&self.path);

        self.active = false;
    }
}

impl std::fmt::Debug for CGroupController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CGroupController")
            .field("container_id", &self.container_id)
            .field("path", &self.path)
            .field("active", &self.active)
            .finish()
    }
}
