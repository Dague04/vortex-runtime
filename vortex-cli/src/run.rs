//! Container execution logic

use anyhow::{Context, Result};
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
    let container_id = ContainerId::new(&args.id).context("Invalid container ID")?;

    info!("📦 Container ID: {}", container_id);

    // === CGROUP SETUP (must happen before fork) ===
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

    // Show initial stats
    info!("📊 Initial statistics:");
    display_stats(&cgroup).await?;

    // === NAMESPACE ISOLATION + COMMAND EXECUTION ===
    let exit_code = if args.isolate && args.pid_isolate {
        // Full isolation with fork/exec
        execute_with_full_isolation(&args, &container_id)?
    } else if args.isolate {
        // Partial isolation without PID namespace
        execute_with_partial_isolation(&args, &container_id).await?
    } else {
        // No isolation (just for testing)
        info!("⚠️  Running without namespace isolation");
        info!("🚀 Would execute: {}", args.command.join(" "));
        wait_for_ctrl_c().await?;
        0
    };

    // Show final stats
    info!("📊 Final statistics:");
    display_stats(&cgroup).await?;

    // Cleanup
    info!("🧹 Cleaning up cgroup...");
    cgroup.cleanup().await.context("Failed to cleanup cgroup")?;

    if exit_code == 0 {
        info!("✅ Container stopped successfully");
    } else {
        warn!("⚠️  Container exited with code: {}", exit_code);
    }

    Ok(())
}

/// Execute with full PID isolation (fork/exec)
/// Execute with full PID isolation (fork/exec)
fn execute_with_full_isolation(args: &RunArgs, container_id: &ContainerId) -> Result<i32> {
    info!("🔒 Setting up FULL namespace isolation (with PID)...");

    let mut ns_config = NamespaceConfig::new();
    ns_config.enable_pid = true; // Enable PID namespace

    // Set hostname
    if let Some(ref hostname) = args.hostname {
        ns_config = ns_config.with_hostname(hostname);
    } else {
        ns_config = ns_config.with_hostname(container_id.as_str());
    }

    // Set rootfs if provided
    if let Some(ref rootfs) = args.rootfs {
        ns_config = ns_config.with_rootfs(rootfs.clone());
    }

    let ns_manager = NamespaceManager::new(ns_config);

    info!("🚀 Executing command in isolated namespace...");
    info!("   Command: {}", args.command.join(" "));

    // Execute command - this will fork and wait
    let exit_code = ns_manager.execute_command(&args.command)?;

    info!("✅ Command completed with exit code: {}", exit_code);

    Ok(exit_code)
}

/// Execute with partial isolation (no PID namespace)
async fn execute_with_partial_isolation(args: &RunArgs, container_id: &ContainerId) -> Result<i32> {
    info!("🔒 Setting up namespace isolation (without PID)...");

    let mut ns_config = NamespaceConfig::new();
    ns_config.enable_pid = false; // ← No PID namespace

    // Set hostname
    if let Some(ref hostname) = args.hostname {
        ns_config = ns_config.with_hostname(hostname);
    } else {
        ns_config = ns_config.with_hostname(container_id.as_str());
    }

    let ns_manager = NamespaceManager::new(ns_config);

    // Enter namespaces
    ns_manager
        .enter_namespaces()
        .context("Failed to enter namespaces")?;

    info!("✅ Namespace isolation enabled");

    if let Ok(hostname) = hostname::get() {
        info!("   Hostname: {}", hostname.to_string_lossy());
    }

    info!("🚀 Would execute: {}", args.command.join(" "));

    // Wait for Ctrl+C
    wait_for_ctrl_c().await?;

    Ok(0)
}

async fn wait_for_ctrl_c() -> Result<()> {
    info!("");
    info!("⏸️  Container running. Press Ctrl+C to stop...");

    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    info!("");
    info!("🛑 Stopping container...");

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
