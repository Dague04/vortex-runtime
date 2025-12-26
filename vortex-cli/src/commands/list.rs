//! List command implementation

use anyhow::{Context, Result};
use vortex_cgroup::{CGroupController, ResourceBackend};
use vortex_core::ContainerId;

pub async fn execute() -> Result<()> {
    tracing::info!("Listing containers");

    println!("\n📋 Containers");
    println!("{:-<60}", "");

    let vortex_path = std::path::Path::new("/sys/fs/cgroup/vortex");

    if !vortex_path.exists() {
        println!("No containers running");
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(vortex_path)
        .await
        .context("Failed to read vortex cgroup directory")?;

    let mut count = 0;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            let name = entry.file_name();
            let id = name.to_string_lossy();

            if let Ok(container_id) = ContainerId::new(id.as_ref()) {
                if let Ok(controller) = CGroupController::new(container_id).await {
                    if let Ok(stats) = controller.stats().await {
                        println!(
                            "  {} - CPU: {:.2}s, Memory: {}",
                            id,
                            stats.cpu_usage.as_secs_f64(),
                            stats.memory_current
                        );
                        count += 1;
                    }
                }
            }
        }
    }

    if count == 0 {
        println!("No containers running");
    } else {
        println!("{:-<60}", "");
        println!("Total: {} container(s)", count);
    }

    Ok(())
}
