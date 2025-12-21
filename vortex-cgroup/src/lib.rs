//! # CGroup v2 Management
//!
//! This crate provides safe, idiomatic Rust bindings for Linux CGroups v2.
//!
//! ## Example
//!
//! ```no_run
//! use vortex_cgroup::CGroupController;
//! use vortex_core::{ContainerId, CpuLimit, CpuCores, MemoryLimit, MemorySize};
//!

// Deny unsafe code - we want memory safety!
#![deny(unsafe_code)]

// Re-export main types
pub use controller::CGroupController;

// Modules
mod controller;
mod limits;
//mod fs;
//mod stats;

/// CGroup v2 base path
pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";

/// Vortex namespace within cgroup hierarchy
pub const VORTEX_NAMESPACE: &str = "vortex";
