//! Namespace configuration

use std::path::PathBuf;

/// Configuration for namespace creation
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    /// Enable PID namespace (process isolation)
    pub enable_pid: bool,

    /// Enable mount namespace (filesystem isolation)
    pub enable_mount: bool,

    /// Enable network namespace (network isolation)
    pub enable_network: bool,

    /// Enable UTS namespace (hostname isolation)
    pub enable_uts: bool,

    /// Enable IPC namespace
    pub enable_ipc: bool,

    /// Container hostname (used with UTS namespace)
    pub hostname: Option<String>,

    /// Path to container root filesystem
    pub rootfs: Option<PathBuf>,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            enable_pid: true,
            enable_mount: true,
            enable_network: false, // Network isolation is complex
            enable_uts: true,
            enable_ipc: true,
            hostname: Some("vortex-container".to_string()),
            rootfs: None,
        }
    }
}

impl NamespaceConfig {
    /// Create a new namespace configuration
    ///
    /// Note: PID namespace is disabled by default because it requires
    /// fork() to take full effect. Enable it only if you plan to fork/exec.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable all namespaces
    pub fn all() -> Self {
        Self {
            enable_pid: true,
            enable_mount: true,
            enable_network: true,
            enable_uts: true,
            enable_ipc: true,
            hostname: Some("vortex-container".to_string()),
            rootfs: None,
        }
    }

    /// Minimal isolation (PID + Mount only)
    pub fn minimal() -> Self {
        Self {
            enable_pid: true,
            enable_mount: true,
            enable_network: false,
            enable_uts: false,
            enable_ipc: false,
            hostname: None,
            rootfs: None,
        }
    }

    /// Set container hostname
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Set container rootfs path
    pub fn with_rootfs(mut self, rootfs: PathBuf) -> Self {
        self.rootfs = Some(rootfs);
        self
    }
}
