//! CLI argument definitions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vortex")]
#[command(about = "Vortex container runtime", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a container
    Run {
        /// Container ID
        #[arg(short, long)]
        id: String,

        /// CPU limit in cores (e.g., 1.0, 0.5)
        #[arg(long, default_value = "1.0")]
        cpu: f64,

        /// Memory limit in MB
        #[arg(long, default_value = "512")]
        memory: u64,

        /// Enable monitoring
        #[arg(long)]
        monitor: bool,

        /// Disable namespaces
        #[arg(long)]
        no_namespaces: bool,

        /// Custom hostname
        #[arg(long)]
        hostname: Option<String>,

        /// Command to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Get container stats
    Stats {
        /// Container ID
        #[arg(short, long)]
        id: String,
    },

    /// List all containers
    List,

    /// Stop a container
    Stop {
        /// Container ID
        #[arg(short, long)]
        id: String,
    },

    /// Show namespace information
    Namespaces {
        /// Process ID (default: current process)
        #[arg(short, long)]
        pid: Option<u32>,
    },
}
