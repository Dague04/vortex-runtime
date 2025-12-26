//! Run command implementation

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::mpsc;
use vortex_cgroup::{CGroupController, ResourceBackend, ResourceMonitor};
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize};
use vortex_namespace::{NamespaceConfig, NamespaceExecutor};

pub async fn execute(
    id: &str,
    cpu: f64,
    memory: u64,
    enable_monitor: bool,
    no_namespaces: bool,
    hostname: Option<String>,
    command: &[String],
) -> Result<()> {
    tracing::info!(container_id = id, "Starting container");

    // Validate we're root
    if !nix::unistd::getuid().is_root() {
        anyhow::bail!("Must run as root (try: sudo vortex run ...)");
    }

    // Create container ID
    let container_id = ContainerId::new(id).context("Invalid container ID")?;

    // Create CGroup controller
    let mut controller = CGroupController::new(container_id.clone())
        .await
        .context("Failed to create CGroup controller")?;

    // Set resource limits
    setup_limits(&controller, cpu, memory).await?;

    print_configuration(id, cpu, memory, command);

    // Setup namespaces
    let ns_config = configure_namespaces(no_namespaces, hostname)?;

    // Start monitoring if requested
    let monitor_handle = if enable_monitor {
        // Clone controller for monitoring (Arc prevents cleanup)
        let monitoring_controller = CGroupController::new(container_id.clone())
            .await
            .context("Failed to create monitoring controller")?;
        Some(start_monitoring(monitoring_controller, container_id.clone()).await?)
    } else {
        None
    };

    // Execute command in namespaces
    println!("\n🚀 Starting container...\n");

    let result = execute_in_namespace(ns_config, command)?;

    print_execution_result(&result);

    // Stop monitoring if enabled
    if let Some((monitor, handle)) = monitor_handle {
        monitor.stop().await;
        handle.await?;
    }

    // Cleanup controller explicitly
    controller
        .cleanup()
        .await
        .context("Failed to cleanup controller")?;

    println!("\n✅ Container stopped");

    Ok(())
}

async fn setup_limits(controller: &CGroupController, cpu: f64, memory: u64) -> Result<()> {
    let cpu_limit = CpuLimit::new(CpuCores::new(cpu));
    controller
        .set_cpu_limit(cpu_limit)
        .await
        .context("Failed to set CPU limit")?;

    let memory_limit = MemoryLimit::new(MemorySize::from_mb(memory));
    controller
        .set_memory_limit(memory_limit)
        .await
        .context("Failed to set memory limit")?;

    Ok(())
}

fn print_configuration(id: &str, cpu: f64, memory: u64, command: &[String]) {
    println!("✅ Container {} configured", id);
    println!("   CPU limit: {} cores", cpu);
    println!("   Memory limit: {} MB", memory);
    println!("   Command: {}", command.join(" "));
}

fn configure_namespaces(no_namespaces: bool, hostname: Option<String>) -> Result<NamespaceConfig> {
    let config = if no_namespaces {
        println!("   Namespaces: disabled");
        NamespaceConfig::new()
            .with_pid(false)
            .with_network(false)
            .with_mount(false)
            .with_uts(false)
    } else {
        let mut config = NamespaceConfig::minimal();
        if let Some(ref host) = hostname {
            config = config.with_uts(true).with_hostname(host);
            println!("   Hostname: {}", host);
        }
        let enabled = config.enabled_namespaces();
        println!("   Namespaces: {}", enabled.join(", "));
        config
    };

    Ok(config)
}

async fn start_monitoring(
    controller: CGroupController,
    container_id: ContainerId,
) -> Result<(ResourceMonitor, tokio::task::JoinHandle<()>)> {
    let backend = Arc::new(controller) as Arc<dyn ResourceBackend>;
    let (tx, mut rx) = mpsc::channel(100);

    let monitor = ResourceMonitor::new(backend, container_id, 2).with_events(tx);

    let handle = monitor.start().await?;

    // Spawn event handler
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            event.emit_trace();
        }
    });

    Ok((monitor, handle))
}

fn execute_in_namespace(
    ns_config: NamespaceConfig,
    command: &[String],
) -> Result<vortex_namespace::executor::ExecutionResult> {
    let mut executor = NamespaceExecutor::new(ns_config);
    let program = &command[0];
    let args = &command[1..];

    // Convert vortex_core::Result to anyhow::Result
    executor
        .execute(program, args)
        .map_err(|e| anyhow::anyhow!("{}", e))
}

fn print_execution_result(result: &vortex_namespace::executor::ExecutionResult) {
    println!("\n📊 Execution completed");
    println!("   Exit code: {}", result.exit_code);

    if !result.stdout.is_empty() {
        println!("\n--- STDOUT ---");
        println!("{}", result.stdout_string());
    }

    if !result.stderr.is_empty() {
        println!("\n--- STDERR ---");
        println!("{}", result.stderr_string());
    }
}
