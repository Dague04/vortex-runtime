//! Namespace lifecycle management

use nix::sched::{unshare, CloneFlags};
use nix::unistd::sethostname;
use vortex_core::{Error, Result};

use crate::config::NamespaceConfig;

/// Namespace manager for creating and managing namespaces
#[derive(Debug)]
pub struct NamespaceManager {
    config: NamespaceConfig,
    created: bool,
}

impl NamespaceManager {
    /// Create a new namespace manager
    #[must_use]
    pub fn new(config: NamespaceConfig) -> Self {
        Self {
            config,
            created: false,
        }
    }

    /// Create a new namespace manager with default config
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(NamespaceConfig::default())
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &NamespaceConfig {
        &self.config
    }

    /// Check if namespaces have been created
    #[must_use]
    pub fn is_created(&self) -> bool {
        self.created
    }

    /// Create the configured namespaces
    ///
    /// This calls unshare(2) to create new namespaces for the current process.
    /// Note: PID namespace isolation requires forking - current process won't have PID 1.
    ///
    /// # Errors
    /// Returns error if namespace creation fails (typically due to permissions)
    pub fn create(&mut self) -> Result<()> {
        if self.created {
            tracing::warn!("Namespaces already created");
            return Ok(());
        }

        if !self.config.has_any() {
            tracing::warn!("No namespaces enabled");
            return Ok(());
        }

        // If PID namespace is requested, we need special handling
        // because you can't unshare PID namespace for current process
        // Only child processes will have new PID namespace
        let mut flags = self.config.to_clone_flags();

        // Remove PID namespace from unshare flags - it only affects children
        let has_pid_ns = flags.contains(CloneFlags::CLONE_NEWPID);
        if has_pid_ns {
            flags.remove(CloneFlags::CLONE_NEWPID);
            tracing::debug!("PID namespace will affect child processes only");
        }

        let enabled = self.config.enabled_namespaces();

        tracing::info!(
            namespaces = ?enabled,
            "Creating namespaces"
        );

        // Create namespaces (except PID which requires fork)
        if !flags.is_empty() {
            unshare(flags).map_err(|e| {
                tracing::error!(
                    error = %e,
                    namespaces = ?enabled,
                    "Failed to create namespaces"
                );
                Error::Namespace {
                    message: format!("Failed to unshare namespaces: {e}"),
                }
            })?;
        }

        tracing::debug!("Namespaces created successfully");

        // Configure UTS namespace if enabled
        if self.config.uts {
            self.setup_uts()?;
        }

        self.created = true;

        if has_pid_ns {
            tracing::info!(
                namespaces = ?enabled,
                "Namespace setup complete (PID namespace will be active in child processes)"
            );
        } else {
            tracing::info!(
                namespaces = ?enabled,
                "Namespace setup complete"
            );
        }

        Ok(())
    }
    fn setup_uts(&self) -> Result<()> {
        // Set hostname if configured
        if let Some(ref hostname) = self.config.hostname {
            tracing::debug!(hostname = %hostname, "Setting hostname");

            sethostname(hostname).map_err(|e| {
                tracing::error!(
                    hostname = %hostname,
                    error = %e,
                    "Failed to set hostname"
                );
                Error::Namespace {
                    message: format!("Failed to set hostname: {e}"),
                }
            })?;
        }

        // Set domain name if configured
        if let Some(ref domainname) = self.config.domainname {
            tracing::debug!(domainname = %domainname, "Setting domain name");

            // Use libc directly since nix doesn't expose setdomainname
            unsafe {
                let c_domainname =
                    std::ffi::CString::new(domainname.as_str()).map_err(|e| Error::Namespace {
                        message: format!("Invalid domain name: {e}"),
                    })?;

                if libc::setdomainname(c_domainname.as_ptr(), domainname.len()) != 0 {
                    let err = std::io::Error::last_os_error();
                    tracing::error!(
                        domainname = %domainname,
                        error = %err,
                        "Failed to set domain name"
                    );
                    return Err(Error::Namespace {
                        message: format!("Failed to set domain name: {err}"),
                    });
                }
            }
        }

        Ok(())
    }
    /// Enter existing namespaces (for joining a container)
    ///
    /// # Errors
    /// Returns error if setns fails
    pub fn enter(&self, _pid: i32) -> Result<()> {
        // TODO: Implement namespace entering with setns(2)
        tracing::warn!("Namespace entering not yet implemented");
        Ok(())
    }

    /// Get current namespace IDs
    ///
    /// # Errors
    /// Returns error if reading namespace IDs fails
    pub fn current_namespaces(&self) -> Result<NamespaceInfo> {
        let pid = std::process::id();
        Self::namespaces_for_pid(pid)
    }

    /// Get namespace IDs for a specific PID
    ///
    /// # Errors
    /// Returns error if reading namespace IDs fails
    pub fn namespaces_for_pid(pid: u32) -> Result<NamespaceInfo> {
        use std::fs;

        let base_path = format!("/proc/{pid}/ns");

        let read_ns = |name: &str| -> Result<String> {
            let path = format!("{base_path}/{name}");
            fs::read_link(&path)
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| Error::Namespace {
                    message: format!("Failed to read {name} namespace: {e}"),
                })
        };

        Ok(NamespaceInfo {
            pid: read_ns("pid").ok(),
            net: read_ns("net").ok(),
            mnt: read_ns("mnt").ok(),
            uts: read_ns("uts").ok(),
            ipc: read_ns("ipc").ok(),
            user: read_ns("user").ok(),
            cgroup: read_ns("cgroup").ok(),
        })
    }
}

/// Information about current namespaces
#[derive(Debug, Clone, Default)]
pub struct NamespaceInfo {
    /// PID namespace ID
    pub pid: Option<String>,
    /// Network namespace ID
    pub net: Option<String>,
    /// Mount namespace ID
    pub mnt: Option<String>,
    /// UTS namespace ID
    pub uts: Option<String>,
    /// IPC namespace ID
    pub ipc: Option<String>,
    /// User namespace ID
    pub user: Option<String>,
    /// CGroup namespace ID
    pub cgroup: Option<String>,
}

impl NamespaceInfo {
    /// Check if in different namespace than init (PID 1)
    ///
    /// # Errors
    /// Returns error if cannot read namespaces
    pub fn is_isolated(&self) -> Result<bool> {
        let init_ns = NamespaceManager::namespaces_for_pid(1)?;

        Ok(self.pid != init_ns.pid || self.net != init_ns.net || self.mnt != init_ns.mnt)
    }
}

impl std::fmt::Display for NamespaceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Namespace Info:")?;
        if let Some(ref pid) = self.pid {
            writeln!(f, "  PID:    {pid}")?;
        }
        if let Some(ref net) = self.net {
            writeln!(f, "  NET:    {net}")?;
        }
        if let Some(ref mnt) = self.mnt {
            writeln!(f, "  MNT:    {mnt}")?;
        }
        if let Some(ref uts) = self.uts {
            writeln!(f, "  UTS:    {uts}")?;
        }
        if let Some(ref ipc) = self.ipc {
            writeln!(f, "  IPC:    {ipc}")?;
        }
        if let Some(ref user) = self.user {
            writeln!(f, "  USER:   {user}")?;
        }
        if let Some(ref cgroup) = self.cgroup {
            writeln!(f, "  CGROUP: {cgroup}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let config = NamespaceConfig::default();
        let manager = NamespaceManager::new(config);

        assert!(!manager.is_created());
        assert!(manager.config().has_any());
    }

    #[test]
    fn test_current_namespaces() {
        let manager = NamespaceManager::with_defaults();
        let ns_info = manager.current_namespaces();

        assert!(ns_info.is_ok());
        let info = ns_info.unwrap();
        assert!(info.pid.is_some());
    }

    #[test]
    fn test_namespace_info_display() {
        let info = NamespaceInfo {
            pid: Some("pid:[4026531836]".to_string()),
            net: Some("net:[4026531905]".to_string()),
            ..Default::default()
        };

        let display = format!("{info}");
        assert!(display.contains("PID:"));
        assert!(display.contains("NET:"));
    }
}
