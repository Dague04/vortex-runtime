use anyhow::Result;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    use clap::Parser;
    let cli = cli::Cli::parse();

    // Setup logging
    let log_level = "info";

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .init();

    // Dispatch command
    commands::dispatch(cli.command).await
}
