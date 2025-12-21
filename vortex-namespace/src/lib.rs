//! Linux namespace isolation
//!
//! This crate provides safe wrappers around Linux namespace APIs:
//! - PID namespace (process isolation)
//! - Mount namespace (filesystem isolation)
//! - Network namespace (network isolation)
//! - UTS namespace (hostname isolation)
//! - IPC namespace (inter-process communication isolation)

#![deny(unsafe_code)]

pub use config::NamespaceConfig;
pub use manager::NamespaceManager;

mod config;
//mod isolation;
mod manager;

/// Available namespace types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceType {
    /// Process ID namespace
    Pid,
    /// Mount namespace (filesystem)
    Mount,
    /// Network namespace
    Network,
    /// UTS namespace (hostname)
    Uts,
    /// IPC namespace
    Ipc,
    /// User namespace
    User,
    /// CGroup namespace
    Cgroup,
}
