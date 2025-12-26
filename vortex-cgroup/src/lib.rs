//! CGroup v2 resource management with pluggable backends
//!
//! This crate provides a trait-based abstraction over CGroup v2 for container
//! resource management, including production and mock implementations.

#![warn(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

pub mod backend;
pub mod controller;
pub mod monitor;

pub use backend::{MockBackend, ResourceBackend};
pub use controller::CGroupController;
pub use monitor::ResourceMonitor;

// Re-export commonly used types
pub use vortex_core::{CpuLimit, MemoryLimit, ResourceStats};
