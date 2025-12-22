//! Namespace manager implementation

use nix::sched::{unshare, CloneFlags};
use nix::unistd;
use tracing::{debug, info};
use vortex_core::{Error, Result};

use crate::config::NamespaceConfig;
use crate::executor;

/// Manages Linux namespaces for container isolation
pub struct NamespaceManager {
    config: NamespaceConfig,
}

impl NamespaceManager {
    /// Create a new namespace manager
    pub fn new(config: NamespaceConfig) -> Self {
        Self { config }
    }

    /// Enter namespaces (unshare from parent)
    ///
    /// This creates new namespaces for the current process.
    /// After this call, the process is isolated according to config.
    ///
    /// # Safety
    ///
    /// This is safe Rust, but modifies process state in ways that affect
    /// the entire process (not just this thread).
    pub fn enter_namespaces(&self) -> Result<()> {
        info!("🔒 Entering namespaces...");

        let mut flags = CloneFlags::empty();

        // Build flags based on config
        if self.config.enable_pid {
            debug!("  • PID namespace");
            flags |= CloneFlags::CLONE_NEWPID;
        }

        if self.config.enable_mount {
            debug!("  • Mount namespace");
            flags |= CloneFlags::CLONE_NEWNS;
        }

        if self.config.enable_network {
            debug!("  • Network namespace");
            flags |= CloneFlags::CLONE_NEWNET;
        }

        if self.config.enable_uts {
            debug!("  • UTS namespace (hostname)");
            flags |= CloneFlags::CLONE_NEWUTS;
        }

        if self.config.enable_ipc {
            debug!("  • IPC namespace");
            flags |= CloneFlags::CLONE_NEWIPC;
        }

        // Unshare - create new namespaces
        unshare(flags).map_err(|e| Error::Namespace {
            message: format!("Failed to unshare namespaces: {}", e),
        })?;

        info!("✅ Namespaces entered");

        // Set hostname if UTS namespace is enabled
        if self.config.enable_uts {
            if let Some(ref hostname) = self.config.hostname {
                self.set_hostname(hostname)?;
            }
        }

        Ok(())
    }

    /// Set container hostname
    fn set_hostname(&self, hostname: &str) -> Result<()> {
        debug!("Setting hostname to: {}", hostname);

        unistd::sethostname(hostname).map_err(|e| Error::Namespace {
            message: format!("Failed to set hostname: {}", e),
        })?;

        debug!("✅ Hostname set");
        Ok(())
    }

    /// Get current namespace configuration
    pub fn config(&self) -> &NamespaceConfig {
        &self.config
    }

    /// Execute a command in isolated namespaces
    ///
    /// This method:
    /// 1. Enters namespaces (including PID)
    /// 2. Forks the process
    /// 3. Child becomes PID 1 in new namespace
    /// 4. Execs the command
    /// 5. Parent waits for child
    ///
    /// Returns the exit code of the command.
    pub fn execute_command(&self, command: &[String]) -> Result<i32> {
        // First, enter all namespaces
        self.enter_namespaces()?;

        // Now fork and exec
        // The child will be PID 1 in the new PID namespace
        executor::execute_in_namespace(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_config() {
        let config = NamespaceConfig::new();
        assert!(config.enable_pid);
        assert!(config.enable_mount);

        let manager = NamespaceManager::new(config);
        assert!(manager.config().enable_pid);
    }

    #[test]
    fn test_config_builder() {
        let config = NamespaceConfig::minimal().with_hostname("test-container");

        assert_eq!(config.hostname, Some("test-container".to_string()));
    }
}
