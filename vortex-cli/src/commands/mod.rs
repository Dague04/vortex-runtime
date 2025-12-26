//! Command handlers

mod list;
mod namespaces;
mod run;
mod stats;
mod stop;

use crate::cli::Commands;
use anyhow::Result;

/// Dispatch commands to their handlers
pub async fn dispatch(command: Commands) -> Result<()> {
    match command {
        Commands::Run {
            id,
            cpu,
            memory,
            monitor,
            no_namespaces,
            hostname,
            command,
        } => run::execute(&id, cpu, memory, monitor, no_namespaces, hostname, &command).await,
        Commands::Stats { id } => stats::execute(&id).await,
        Commands::List => list::execute().await,
        Commands::Stop { id } => stop::execute(&id).await,
        Commands::Namespaces { pid } => namespaces::execute(pid).await,
    }
}
