//! CGroup controller implementation

use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};
use vortex_core::{ContainerId, Error, ProcessId, Result};

/// Main CGroup controller
///
/// This struct represents a CGroup in the filesystem hierarchy.
/// It provides methods to:
/// - Set resource limits (CPU, memory, I/O)
/// - Add/remove processes
/// - Read statistics
/// - Clean up on drop

pub struct CGroupController {
    /// Container ID (used for cgroup name)
    pub(crate) container_id: ContainerId,

    /// Full path to this group directory
    /// example: /sys/fs/cgroup/vortex/my-container
    pub(crate) path: PathBuf,

    /// Whether this controller is active
    pub(crate) active: bool,
}

impl CGroupController {
    /// Create a new CGroup controller
    ///
    /// This will:
    /// 1. Build the cgroup path
    /// 2. Create the directory hierarchy
    /// 3. Enable necessary controllers
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Not running as root
    /// - CGroup v2 not available
    /// - Directory creation fails
    pub async fn new(container_id: ContainerId) -> Result<Self> {
        // Build path: /sys/fs/cgroup/vortex/{container_id}
        let path = PathBuf::from(crate::CGROUP_ROOT)
            .join(crate::VORTEX_NAMESPACE)
            .join(container_id.as_str());

        debug!("Creating cgroup at: {}", path.display());

        // Create the struct
        let mut controller = Self {
            container_id,
            path,
            active: false,
        };

        // Actually create the cgroup on filesystem
        controller.create().await?;
        Ok(controller)
    }

    /// Get the container ID
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }

    /// Create the cgroup directory structure
    async fn create(&mut self) -> Result<()> {
        // Step 1: Ensure parent directory exists
        let parent = self
            .path
            .parent()
            .ok_or_else(|| Error::InvalidConfig("Invalid cgroup path".to_string()))?;

        // Create parent if it doesn't exist
        if !parent.exists() {
            debug!("Creating parent directory: {}", parent.display());
            fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::PermissionDenied {
                    operation: format!("Create parent directory: {}", e),
                })?;
        }

        // Step 2: Enable controllers in parent (NOW parent exists!)
        self.enable_controllers(parent).await?;

        // Step 3: Create our cgroup directory
        if !self.path.exists() {
            debug!("Creating cgroup directory: {}", self.path.display());
            fs::create_dir(&self.path)
                .await
                .map_err(|e| Error::CGroup {
                    message: format!("Failed to create cgroup directory: {}", e),
                })?;
        }

        // Step 4: Verify it exists
        if !self.path.exists() {
            return Err(Error::CGroup {
                message: "CGroup directory was not created".to_string(),
            });
        }

        self.active = true;
        debug!("CGroup created successfully: {}", self.path.display());
        Ok(())
    }

    /// Enable necessary controllers in parent cgroup
    /// Enable necessary controllers in parent cgroup
    async fn enable_controllers(&self, parent: &std::path::Path) -> Result<()> {
        let control_file = parent.join("cgroup.subtree_control");

        // Read current state
        let current = match fs::read_to_string(&control_file).await {
            Ok(content) => content,
            Err(_) => return Ok(()), // Can't read? Skip enabling
        };

        // Check if all needed controllers are present
        let needed = ["cpu", "memory", "io"];
        let all_enabled = needed.iter().all(|c| current.contains(c));

        if all_enabled {
            debug!("All required controllers already enabled");
            return Ok(());
        }

        // Try to enable missing controllers (but don't fail)
        let missing: Vec<String> = needed
            .iter()
            .filter(|c| !current.contains(*c))
            .map(|c| format!("+{}", c))
            .collect();

        if !missing.is_empty() {
            debug!("Attempting to enable: {}", missing.join(" "));
            let _ = fs::write(&control_file, missing.join(" ")).await;
        }

        Ok(())
    }

    pub async fn add_process(&self, pid: ProcessId) -> Result<()> {
        if !self.active {
            return Err(Error::CGroup {
                message: "CGroup not active".to_string(),
            });
        }

        let procs_file = self.path.join("cgroup.procs");
        let pid_str = pid.as_raw().to_string();

        debug!("Adding process {} to cgroup", pid);

        fs::write(&procs_file, pid_str)
            .await
            .map_err(|e| Error::PermissionDenied {
                operation: format!("Add process to cgroup: {}", e),
            })?;

        Ok(())
    }

    pub async fn cleanup(mut self) -> Result<()> {
        // ... async cleanup ...
        self.active = false; // Mark as cleaned up
        Ok(())
    }
}

impl Drop for CGroupController {
    /// Fallback cleanup (if explicit cleanup wasn't called)
    fn drop(&mut self) {
        if !self.active {
            return; // Already cleaned up
        }

        // Do best-effort blocking cleanup
        warn!("CGroup not explicitly cleaned up, using Drop fallback");
        // ... blocking cleanup ...
    }
}
