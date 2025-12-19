//! # Vortex Core
//!
//! Core types and abstractions for the Vortex container runtime.
//!
//! ## Features
//!
//! - Strongly-typed resource specifications
//! - Ergonomic error handling with `thiserror`
//! - Serializable types for configuration
//!
//! ## Example
//!
//! ```rust
//! use vortex_core::{ContainerId, MemorySize, CpuCores};
//!
//! let id = ContainerId::new("my-container").unwrap();
//! let memory = MemorySize::from_mb(512);
//! let cpu = CpuCores::new(2.0);
//!
//! println!("Container: {}", id);
//! println!("Memory: {}", memory);
//! println!("CPU: {}", cpu);
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]

// Public API
pub use error::{Error, Result};
pub use resources::{
    Bandwidth, CpuCores, CpuLimit, MemoryLimit, MemorySize, ResourceStats,
};
pub use types::{ContainerId, ProcessId};

// Modules
pub mod error;
pub mod resources;
pub mod types;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");