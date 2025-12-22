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
        // Ensure root controllers are enabled first
        Self::ensure_root_controllers().await?;

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

        debug!("Checking controllers at: {}", control_file.display());

        // Check if file exists
        if !control_file.exists() {
            debug!("Control file does not exist, skipping controller setup");
            return Ok(());
        }

        // Try to read what's currently enabled
        let current = match fs::read_to_string(&control_file).await {
            Ok(content) => {
                debug!("Current controllers: {}", content.trim());
                content
            }
            Err(e) => {
                debug!("Could not read control file: {}", e);
                String::new() // ← Changed! Assume empty instead of returning
            }
        };

        // Define what we need
        let needed = ["cpu", "memory", "io"];

        // Check what's missing
        let missing: Vec<&str> = needed
            .iter()
            .filter(|&&controller| !current.contains(controller))
            .copied()
            .collect();

        // If nothing is missing, we're done
        if missing.is_empty() {
            debug!(
                "All required controllers already enabled in {}",
                parent.display()
            );
            return Ok(());
        }

        // Try to enable missing controllers
        let to_enable: String = missing
            .iter()
            .map(|c| format!("+{}", c))
            .collect::<Vec<_>>()
            .join(" ");

        debug!(
            "Enabling controllers in {}: {}",
            parent.display(),
            to_enable
        );

        // Write to enable controllers
        match fs::write(&control_file, &to_enable).await {
            Ok(_) => {
                debug!("Successfully enabled controllers in {}", parent.display());
                Ok(())
            }
            Err(e) => {
                // Log but don't fail - they might be enabled at a higher level
                debug!(
                    "Could not enable controllers in {}: {}",
                    parent.display(),
                    e
                );

                // If the error is permission denied, that's usually OK
                // (controllers managed at higher level)
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    debug!("Permission denied is OK - controllers may be managed at higher level");
                    Ok(())
                } else {
                    // For other errors, return the error
                    Err(Error::PermissionDenied {
                        operation: format!("Enable controllers in {}: {}", parent.display(), e),
                    })
                }
            }
        }
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

    /// Ensure controllers are enabled at root cgroup level
    async fn ensure_root_controllers() -> Result<()> {
        let root_control = std::path::Path::new("/sys/fs/cgroup/cgroup.subtree_control");

        debug!("Checking root cgroup controllers");

        // Read what's enabled at root
        let current = match fs::read_to_string(root_control).await {
            Ok(content) => content,
            Err(e) => {
                debug!("Could not read root cgroup controllers: {}", e);
                return Ok(()); // Root might be managed by system
            }
        };

        // Check if our needed controllers are enabled
        let needed = ["cpu", "memory", "io"];
        let missing: Vec<&str> = needed
            .iter()
            .filter(|&&c| !current.contains(c))
            .copied()
            .collect();

        if missing.is_empty() {
            debug!("All controllers already enabled at root");
            return Ok(());
        }

        // Try to enable missing controllers
        let to_enable: String = missing
            .iter()
            .map(|c| format!("+{}", c))
            .collect::<Vec<_>>()
            .join(" ");

        debug!("Attempting to enable at root: {}", to_enable);

        match fs::write(root_control, &to_enable).await {
            Ok(_) => {
                debug!("Successfully enabled controllers at root");
                Ok(())
            }
            Err(e) => {
                debug!("Could not enable root controllers (may be OK): {}", e);
                Ok(()) // Don't fail - system may manage this
            }
        }
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
