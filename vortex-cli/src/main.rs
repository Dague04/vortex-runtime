//! Vortex Container Runtime CLI
//!
//! A lightweight container runtime demonstrating modern Rust and Linux features.

use clap::Parser;
use std::process;
use tracing::Level;

mod cli;
mod run;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Setup logging based on verbosity
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Execute the command
    let result = match cli.command {
        Commands::Run(args) => run::execute(args).await,
        Commands::Version => {
            print_version();
            Ok(())
        }
    };

    // Handle errors
    if let Err(e) = result {
        eprintln!("❌ Error: {}", e);
        process::exit(1);
    }
}

fn print_version() {
    println!("🦀 Vortex Container Runtime");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Edition: Rust 2024");
    println!();
    println!("Features:");
    println!("  • CGroup v2 resource limits");
    println!("  • Linux namespace isolation");
    println!("  • Security hardening");
    println!("  • Real-time monitoring");
    println!();
    println!("Built with ❤️  in Rust");
}
