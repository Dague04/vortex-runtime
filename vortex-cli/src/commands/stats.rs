//! Stats command implementation

use anyhow::{Context, Result};
use vortex_cgroup::{CGroupController, ResourceBackend};
use vortex_core::ContainerId;

pub async fn execute(id: &str) -> Result<()> {
    tracing::info!(container_id = id, "Getting stats");

    let container_id = ContainerId::new(id).context("Invalid container ID")?;

    let controller = CGroupController::new(container_id)
        .await
        .context("Failed to create controller (is container running?)")?;

    let stats = controller.stats().await.context("Failed to read stats")?;

    println!("\n📊 Container Stats for '{}'", id);
    println!("{:-<60}", "");
    println!("CPU Usage:       {:.2}s", stats.cpu_usage.as_secs_f64());
    println!("CPU Throttled:   {:.2}s", stats.cpu_throttled.as_secs_f64());
    println!("Memory Current:  {}", stats.memory_current);
    println!("Memory Peak:     {}", stats.memory_peak);
    println!("Swap Current:    {}", stats.swap_current);
    println!("Swap Peak:       {}", stats.swap_peak);
    println!("I/O Read:        {} bytes", stats.io_read_bytes);
    println!("I/O Write:       {} bytes", stats.io_write_bytes);
    println!("{:-<60}", "");

    Ok(())
}
