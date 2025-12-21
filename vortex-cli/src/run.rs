//! Container execution logic

use anyhow::{Context, Result};
use tokio::signal;
use tracing::{debug, info, warn};
use vortex_cgroup::CGroupController;
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize};

use crate::cli::RunArgs;

pub async fn execute(args: RunArgs) -> Result<()> {
    info!("🦀 Starting Vortex Container Runtime");

    // Validate we're running as root
    if !nix::unistd::geteuid().is_root() {
        anyhow::bail!("Must run as root. Try: sudo vortex run ...");
    }

    // Parse and validate container ID
    let container_id = ContainerId::new(args.id).context("Invalid container ID")?;

    info!("📦 Container ID: {}", container_id);

    // Create CGroup
    info!("🔧 Creating cgroup...");
    let cgroup = CGroupController::new(container_id.clone())
        .await
        .context("Failed to create cgroup")?;

    debug!("✅ CGroup created");

    // Set CPU limit if specified
    if let Some(cpu_cores) = args.cpu {
        validate_cpu_limit(cpu_cores)?;

        let limit = CpuLimit::new(CpuCores::new(cpu_cores));
        info!("⚙️  Setting CPU limit: {:.2} cores", cpu_cores);

        cgroup
            .set_cpu_limit(limit)
            .await
            .context("Failed to set CPU limit")?;

        debug!("✅ CPU limit set");
    } else {
        info!("⚙️  No CPU limit specified (unlimited)");
    }

    // Set memory limit if specified
    if let Some(memory_mb) = args.memory {
        validate_memory_limit(memory_mb)?;

        let limit = MemoryLimit::new(MemorySize::from_mb(memory_mb));
        info!("💾 Setting memory limit: {} MB", memory_mb);

        cgroup
            .set_memory_limit(limit)
            .await
            .context("Failed to set memory limit")?;

        debug!("✅ Memory limit set");
    } else {
        info!("💾 No memory limit specified (unlimited)");
    }

    // Add current process to cgroup
    info!("🔗 Adding process to cgroup...");
    let pid = vortex_core::ProcessId::current();
    cgroup
        .add_process(pid)
        .await
        .context("Failed to add process to cgroup")?;

    info!("✅ Process {} added to cgroup", pid);

    // Display what command would run (for now)
    info!("🚀 Would execute: {}", args.command.join(" "));

    if let Some(rootfs) = args.rootfs {
        info!("📁 With rootfs: {}", rootfs.display());
    } else {
        info!("📁 Using host filesystem (no rootfs specified)");
    }

    // Show current stats
    info!("📊 Initial statistics:");
    display_stats(&cgroup).await?;

    // Wait for Ctrl+C
    info!("");
    info!("⏸️  Container running. Press Ctrl+C to stop...");

    signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    // Cleanup
    info!("");
    info!("🛑 Stopping container...");

    // Show final stats
    info!("📊 Final statistics:");
    display_stats(&cgroup).await?;

    info!("🧹 Cleaning up cgroup...");
    cgroup.cleanup().await.context("Failed to cleanup cgroup")?;

    info!("✅ Container stopped successfully");

    Ok(())
}

/// Validate CPU limit is reasonable
fn validate_cpu_limit(cores: f64) -> Result<()> {
    if cores <= 0.0 {
        anyhow::bail!("CPU limit must be positive, got: {}", cores);
    }

    if cores > 128.0 {
        anyhow::bail!("CPU limit too high (max 128 cores), got: {}", cores);
    }

    Ok(())
}

/// Validate memory limit is reasonable
fn validate_memory_limit(mb: u64) -> Result<()> {
    if mb == 0 {
        anyhow::bail!("Memory limit must be positive");
    }

    if mb > 1_048_576 {
        anyhow::bail!("Memory limit too high (max 1TB), got: {} MB", mb);
    }

    Ok(())
}

/// Display current resource statistics
async fn display_stats(cgroup: &CGroupController) -> Result<()> {
    match cgroup.stats().await {
        Ok(stats) => {
            info!(
                "  CPU usage: {:.2} ms",
                stats.cpu_usage.as_secs_f64() * 1000.0
            );
            info!(
                "  CPU throttled: {:.2} ms",
                stats.cpu_throttled.as_secs_f64() * 1000.0
            );
            info!("  Memory current: {}", stats.memory_current);
            info!("  Memory peak: {}", stats.memory_peak);
            info!("  I/O read: {:.2} KB", stats.io_read_bytes as f64 / 1024.0);
            info!(
                "  I/O write: {:.2} KB",
                stats.io_write_bytes as f64 / 1024.0
            );
            Ok(())
        }
        Err(e) => {
            warn!("Could not read stats: {}", e);
            Ok(())
        }
    }
}
