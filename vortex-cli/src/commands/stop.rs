//! Stop command implementation

use anyhow::{Context, Result};
use vortex_cgroup::CGroupController;
use vortex_core::ContainerId;

pub async fn execute(id: &str) -> Result<()> {
    tracing::info!(container_id = id, "Stopping container");

    let container_id = ContainerId::new(id).context("Invalid container ID")?;

    let mut controller = CGroupController::new(container_id)
        .await
        .context("Failed to access container (is it running?)")?;

    controller
        .cleanup()
        .await
        .context("Failed to cleanup container")?;

    println!("✅ Container '{}' stopped", id);

    Ok(())
}
