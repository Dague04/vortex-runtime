//! Vortex Core - Foundation types, events, and utilities
//!
//! This crate provides the core abstractions used throughout Vortex.

#![warn(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod events;
pub mod resources;
pub mod types;

pub use error::{Error, Result};
pub use events::ContainerEvent;
pub use resources::{CpuCores, CpuLimit, MemoryLimit, MemorySize, ResourceStats};
pub use types::{ContainerId, ProcessId};
