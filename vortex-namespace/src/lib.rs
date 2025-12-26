//! Namespace management for process isolation
//!
//! This crate provides Linux namespace isolation for containers:
//! - PID namespace - Process isolation
//! - Network namespace - Network isolation
//! - Mount namespace - Filesystem isolation
//! - UTS namespace - Hostname isolation
//! - IPC namespace - Inter-process communication isolation
//! - User namespace - UID/GID mapping

#![warn(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

pub mod config;
pub mod executor;
pub mod manager;

pub use config::{NamespaceConfig, NamespaceFlags};
pub use executor::NamespaceExecutor;
pub use manager::NamespaceManager;
