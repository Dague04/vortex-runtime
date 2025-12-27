use clap::{Parser, Subcommand};

/// Vortex container runtime
#[derive(Parser, Debug)]
#[command(name = "vortex")]
#[command(about = "Lightweight container runtime", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a container
    Run {
        /// Container ID
        #[arg(short, long)]
        id: String,

        /// CPU limit in cores (default: 1.0)
        #[arg(long, default_value = "1.0")]
        cpu: f64,

        /// Memory limit in MB (default: 512)
        #[arg(long, default_value = "512")]
        memory: u64,

        /// Enable resource monitoring
        #[arg(long)]
        monitor: bool,

        /// Disable namespaces (no isolation)
        #[arg(long)]
        no_namespaces: bool,

        /// Container hostname
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
        /// Process ID to inspect (default: current process)
        #[arg(long)]
        pid: Option<i32>,
    },

    /// Check system health and requirements
    Health,
}
