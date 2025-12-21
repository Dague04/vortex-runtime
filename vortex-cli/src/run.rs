//! Container execution logic

use anyhow::{Context, Result};
use tokio::signal;
use tracing::{debug, info, warn};
use vortex_cgroup::CGroupController;
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize};
use vortex_namespace::{NamespaceConfig, NamespaceManager};

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

    // === CGROUP FIRST (before namespaces!) ===
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

    // === NOW ENTER NAMESPACES (after async setup) ===
    if args.isolate {
        info!("🔒 Setting up namespace isolation...");

        // Disable PID namespace for now (requires fork to work properly)
        let mut ns_config = NamespaceConfig::new();
        ns_config.enable_pid = false; // ← Disable problematic PID namespace

        // Set hostname if provided
        if let Some(hostname) = args.hostname.clone() {
            ns_config = ns_config.with_hostname(hostname);
        } else {
            // Use container ID as hostname
            ns_config = ns_config.with_hostname(container_id.as_str());
        }

        // Set rootfs if provided
        if let Some(rootfs) = args.rootfs.clone() {
            ns_config = ns_config.with_rootfs(rootfs);
        }

        let ns_manager = NamespaceManager::new(ns_config);

        // Enter namespaces (no more async after this point is safest)
        ns_manager
            .enter_namespaces()
            .context("Failed to enter namespaces")?;

        info!("✅ Namespace isolation enabled");

        // Show what we isolated
        if let Ok(hostname) = hostname::get() {
            info!("  Hostname: {}", hostname.to_string_lossy());
        }
    } else {
        info!("⚠️  Namespace isolation disabled");
    }

    // Display what command would run (for now)
    info!("🚀 Would execute: {}", args.command.join(" "));

    if args.rootfs.is_some() {
        info!(
            "📁 With rootfs: {}",
            args.rootfs.as_ref().unwrap().display()
        );
    } else {
        info!("📁 Using host filesystem (no rootfs specified)");
    }

    // Show current stats
    info!("📊 Initial statistics:");
    display_stats(&cgroup).await?;

    // Show isolation status
    info!("");
    info!("🔐 Isolation Summary:");
    if args.isolate {
        info!("  ✅ Namespaces: ENABLED");
        info!("     • Mount isolation");
        info!("     • UTS isolation (hostname)");
        info!("     • IPC isolation");
        info!("     ⚠️  PID isolation (disabled - requires fork)");
    } else {
        info!("  ⚠️  Namespaces: DISABLED");
    }
    if args.cpu.is_some() || args.memory.is_some() {
        info!("  ✅ Resource limits: ENABLED");
    } else {
        info!("  ⚠️  Resource limits: NONE");
    }

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
