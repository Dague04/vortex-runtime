//! Command-line interface definitions using Clap

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Vortex Container Runtime - Lightweight Linux containers in Rust
#[derive(Parser, Debug)]
#[command(name = "vortex")]
#[command(author = "Your Name")]
#[command(version)]
#[command(about = "A lightweight container runtime written in Rust", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose logging (debug level)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a container with specified resource limits
    Run(RunArgs),

    /// Show version information
    Version,
}

/// Arguments for the 'run' command
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Container ID (must be unique)
    #[arg(short, long, default_value = "vortex-default")]
    pub id: String,

    /// CPU limit in cores (e.g., 0.5, 1.0, 2.0)
    #[arg(long)]
    pub cpu: Option<f64>,

    /// Memory limit in megabytes (e.g., 128, 512, 1024)
    #[arg(long)]
    pub memory: Option<u64>,

    /// Path to container rootfs directory
    #[arg(long)]
    pub rootfs: Option<PathBuf>,

    /// Command to execute in container
    #[arg(last = true, default_value = "/bin/sh")]
    pub command: Vec<String>,
}
