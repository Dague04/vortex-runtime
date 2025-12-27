use crate::cli::Commands;
use anyhow::Result;

pub mod health;
pub mod list;
pub mod namespaces;
pub mod run;
pub mod stats;
pub mod stop;

/// Dispatch command to appropriate handler
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

        Commands::Namespaces { pid } => {
            // Convert i32 to u32 for pid
            let pid_u32 = pid.map(|p| p as u32);
            namespaces::execute(pid_u32).await
        }

        Commands::Health => health::execute().await,
    }
}
