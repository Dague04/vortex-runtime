//! Namespace configuration

use nix::sched::CloneFlags;
use serde::{Deserialize, Serialize};

/// Namespace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceConfig {
    /// Enable PID namespace
    pub pid: bool,

    /// Enable network namespace
    pub network: bool,

    /// Enable mount namespace
    pub mount: bool,

    /// Enable UTS namespace (hostname)
    pub uts: bool,

    /// Enable IPC namespace
    pub ipc: bool,

    /// Enable user namespace
    pub user: bool,

    /// Enable cgroup namespace
    pub cgroup: bool,

    /// Hostname for UTS namespace
    pub hostname: Option<String>,

    /// Domain name for UTS namespace
    pub domainname: Option<String>,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            pid: true,
            network: true,
            mount: true,
            uts: true,
            ipc: true,
            user: false, // Requires additional setup
            cgroup: true,
            hostname: None,
            domainname: None,
        }
    }
}

impl NamespaceConfig {
    /// Create a new namespace configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable all namespaces (except user by default)
    #[must_use]
    pub fn all() -> Self {
        Self {
            pid: true,
            network: true,
            mount: true,
            uts: true,
            ipc: true,
            user: false,
            cgroup: true,
            hostname: None,
            domainname: None,
        }
    }

    /// Minimal isolation (only PID and mount)
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            pid: true,
            network: false,
            mount: true,
            uts: false,
            ipc: false,
            user: false,
            cgroup: false,
            hostname: None,
            domainname: None,
        }
    }

    /// Enable PID namespace
    #[must_use]
    pub fn with_pid(mut self, enable: bool) -> Self {
        self.pid = enable;
        self
    }

    /// Enable network namespace
    #[must_use]
    pub fn with_network(mut self, enable: bool) -> Self {
        self.network = enable;
        self
    }

    /// Enable mount namespace
    #[must_use]
    pub fn with_mount(mut self, enable: bool) -> Self {
        self.mount = enable;
        self
    }

    /// Enable UTS namespace
    #[must_use]
    pub fn with_uts(mut self, enable: bool) -> Self {
        self.uts = enable;
        self
    }

    /// Enable IPC namespace
    #[must_use]
    pub fn with_ipc(mut self, enable: bool) -> Self {
        self.ipc = enable;
        self
    }

    /// Enable user namespace
    #[must_use]
    pub fn with_user(mut self, enable: bool) -> Self {
        self.user = enable;
        self
    }

    /// Enable cgroup namespace
    #[must_use]
    pub fn with_cgroup(mut self, enable: bool) -> Self {
        self.cgroup = enable;
        self
    }

    /// Set hostname for UTS namespace
    #[must_use]
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Set domain name for UTS namespace
    #[must_use]
    pub fn with_domainname(mut self, domainname: impl Into<String>) -> Self {
        self.domainname = Some(domainname.into());
        self
    }

    /// Convert to clone flags for unshare(2)
    #[must_use]
    pub fn to_clone_flags(&self) -> CloneFlags {
        let mut flags = CloneFlags::empty();

        if self.pid {
            flags |= CloneFlags::CLONE_NEWPID;
        }
        if self.network {
            flags |= CloneFlags::CLONE_NEWNET;
        }
        if self.mount {
            flags |= CloneFlags::CLONE_NEWNS;
        }
        if self.uts {
            flags |= CloneFlags::CLONE_NEWUTS;
        }
        if self.ipc {
            flags |= CloneFlags::CLONE_NEWIPC;
        }
        if self.user {
            flags |= CloneFlags::CLONE_NEWUSER;
        }
        if self.cgroup {
            flags |= CloneFlags::CLONE_NEWCGROUP;
        }

        flags
    }

    /// Check if any namespaces are enabled
    #[must_use]
    pub fn has_any(&self) -> bool {
        self.pid || self.network || self.mount || self.uts || self.ipc || self.user || self.cgroup
    }

    /// Get list of enabled namespace names
    #[must_use]
    pub fn enabled_namespaces(&self) -> Vec<&'static str> {
        let mut namespaces = Vec::new();

        if self.pid {
            namespaces.push("pid");
        }
        if self.network {
            namespaces.push("net");
        }
        if self.mount {
            namespaces.push("mnt");
        }
        if self.uts {
            namespaces.push("uts");
        }
        if self.ipc {
            namespaces.push("ipc");
        }
        if self.user {
            namespaces.push("user");
        }
        if self.cgroup {
            namespaces.push("cgroup");
        }

        namespaces
    }
}

/// Namespace flags for bitwise operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamespaceFlags(u32);

impl NamespaceFlags {
    /// PID namespace flag
    pub const PID: Self = Self(0b0000_0001);
    /// Network namespace flag
    pub const NET: Self = Self(0b0000_0010);
    /// Mount namespace flag
    pub const MNT: Self = Self(0b0000_0100);
    /// UTS namespace flag
    pub const UTS: Self = Self(0b0000_1000);
    /// IPC namespace flag
    pub const IPC: Self = Self(0b0001_0000);
    /// User namespace flag
    pub const USER: Self = Self(0b0010_0000);
    /// CGroup namespace flag
    pub const CGROUP: Self = Self(0b0100_0000);

    /// All namespaces
    pub const ALL: Self = Self(0b0111_1111);
    /// No namespaces
    pub const NONE: Self = Self(0);

    /// Create from raw value
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Get raw value
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Check if flag is set
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for NamespaceFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for NamespaceFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NamespaceConfig::default();
        assert!(config.pid);
        assert!(config.network);
        assert!(config.mount);
        assert!(!config.user);
    }

    #[test]
    fn test_builder_pattern() {
        let config = NamespaceConfig::new()
            .with_pid(true)
            .with_network(false)
            .with_hostname("test-container");

        assert!(config.pid);
        assert!(!config.network);
        assert_eq!(config.hostname.as_deref(), Some("test-container"));
    }

    #[test]
    fn test_clone_flags_conversion() {
        let config = NamespaceConfig::new().with_pid(true).with_network(true);

        let flags = config.to_clone_flags();
        assert!(flags.contains(CloneFlags::CLONE_NEWPID));
        assert!(flags.contains(CloneFlags::CLONE_NEWNET));
    }

    #[test]
    fn test_enabled_namespaces() {
        let config = NamespaceConfig::minimal();
        let enabled = config.enabled_namespaces();

        assert!(enabled.contains(&"pid"));
        assert!(enabled.contains(&"mnt"));
        assert!(!enabled.contains(&"net"));
    }

    #[test]
    fn test_namespace_flags() {
        let flags = NamespaceFlags::PID | NamespaceFlags::NET;

        assert!(flags.contains(NamespaceFlags::PID));
        assert!(flags.contains(NamespaceFlags::NET));
        assert!(!flags.contains(NamespaceFlags::MNT));
    }
}
