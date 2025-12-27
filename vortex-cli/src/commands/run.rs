use anyhow::{Context, Result};
use std::sync::Arc;
use vortex_cgroup::{CGroupController, ResourceBackend, ResourceMonitor};
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize};
use vortex_namespace::{NamespaceConfig, NamespaceExecutor};

/// Execute the run command
pub async fn execute(
    id: &str,
    cpu: f64,
    memory: u64,
    enable_monitor: bool,
    no_namespaces: bool,
    hostname: Option<String>,
    command: &[String],
) -> Result<()> {
    // Validate environment
    validate_environment()?;

    // Create container ID
    let container_id = create_container_id(id)?;

    // Setup CGroup controller with resource limits
    let controller = setup_cgroup_controller(&container_id, cpu, memory).await?;

    // Setup namespace configuration
    let ns_config = setup_namespace_config(no_namespaces, hostname)?;

    // Display configuration to user
    display_configuration(id, cpu, memory, command, &ns_config);

    // Start monitoring if requested
    let monitor_handle = if enable_monitor {
        Some(start_monitoring(&container_id).await?)
    } else {
        None
    };

    // Execute command in isolated namespace
    println!("\n🚀 Starting container...\n");
    let result = execute_in_namespace(ns_config, command)?;

    // Display execution results
    display_execution_results(&result);

    // Stop monitoring if it was enabled
    if let Some((monitor, handle)) = monitor_handle {
        stop_monitoring(monitor, handle).await?;
    }

    // Cleanup CGroup controller
    controller
        .cleanup()
        .await
        .context("Failed to cleanup controller")?;

    println!("\n✅ Container stopped");

    Ok(())
}

/// Check if running as root
fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

/// Validate that the environment is suitable for running containers
fn validate_environment() -> Result<()> {
    // Check if running as root
    if !is_root() {
        anyhow::bail!(
            "🔒 Permission Denied\n\
             \n\
             Vortex needs root permissions to:\n\
             • Create cgroups (resource limits)\n\
             • Create namespaces (isolation)\n\
             • Access kernel files\n\
             \n\
             Please run with sudo:\n\
             $ sudo vortex run ..."
        );
    }

    // Check if CGroup v2 is available
    let cgroup_root = std::path::Path::new("/sys/fs/cgroup");
    if !cgroup_root.exists() {
        anyhow::bail!(
            "❌ CGroup v2 Not Found\n\
             \n\
             CGroup filesystem not mounted at /sys/fs/cgroup\n\
             \n\
             Check if CGroup v2 is enabled:\n\
             $ mount | grep cgroup2\n\
             \n\
             On most modern Linux distributions, this should be automatic.\n\
             If not, you may need to enable it in your kernel boot parameters."
        );
    }

    Ok(())
}

/// Create and validate container ID
fn create_container_id(id: &str) -> Result<ContainerId> {
    ContainerId::new(id).context("Invalid container ID")
}

/// Setup CGroup controller with resource limits
async fn setup_cgroup_controller(
    container_id: &ContainerId,
    cpu: f64,
    memory: u64,
) -> Result<CGroupController> {
    // Create controller
    let controller = CGroupController::new(container_id.clone())
        .await
        .context("Failed to create CGroup controller")?;

    // Set CPU limit
    let cpu_limit = CpuLimit::new(CpuCores::new(cpu));
    controller
        .set_cpu_limit(cpu_limit)
        .await
        .context("Failed to set CPU limit")?;

    // Set memory limit
    let memory_limit = MemoryLimit::new(MemorySize::from_mb(memory));
    controller
        .set_memory_limit(memory_limit)
        .await
        .context("Failed to set memory limit")?;

    Ok(controller)
}

/// Setup namespace configuration
fn setup_namespace_config(
    no_namespaces: bool,
    hostname: Option<String>,
) -> Result<NamespaceConfig> {
    if no_namespaces {
        return Ok(NamespaceConfig::new());
    }

    let mut config = NamespaceConfig::minimal();

    if let Some(h) = hostname {
        config = config.with_hostname(h);
    }

    Ok(config)
}

/// Display container configuration to user
fn display_configuration(
    id: &str,
    cpu: f64,
    memory: u64,
    command: &[String],
    ns_config: &NamespaceConfig,
) {
    println!("\n✅ Container {} configured", id);
    println!("   CPU limit: {} cores", cpu);
    println!("   Memory limit: {} MB", memory);
    println!("   Command: {}", command.join(" "));

    // Access hostname field directly
    if let Some(ref hostname) = ns_config.hostname {
        println!("   Hostname: {}", hostname);
    }

    if ns_config.has_any() {
        let enabled = ns_config.enabled_namespaces();
        println!("   Namespaces: {}", enabled.join(", "));
    } else {
        println!("   Namespaces: disabled");
    }
}

/// Start resource monitoring for the container
async fn start_monitoring(
    container_id: &ContainerId,
) -> Result<(ResourceMonitor, tokio::task::JoinHandle<()>)> {
    // Create separate controller for monitoring
    // (We can't use the main controller because it needs to be moved for cleanup)
    let monitoring_controller = CGroupController::new(container_id.clone())
        .await
        .context("Failed to create monitoring controller")?;

    let backend: Arc<dyn ResourceBackend> = Arc::new(monitoring_controller);

    let monitor = ResourceMonitor::new(
        backend,
        container_id.clone(),
        2, // Poll every 2 seconds
    );

    let handle = monitor
        .start()
        .await
        .context("Failed to start monitoring")?;

    Ok((monitor, handle))
}

/// Execute command in isolated namespace
fn execute_in_namespace(
    ns_config: NamespaceConfig,
    command: &[String],
) -> Result<vortex_namespace::ExecutionResult> {
    if command.is_empty() {
        anyhow::bail!("No command specified");
    }

    let program = &command[0];
    let args = &command[1..];

    let executor = NamespaceExecutor::new(ns_config)
        .map_err(|e| anyhow::anyhow!("Failed to create executor: {}", e))?;

    executor
        .execute(program, args)
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))
}

/// Display execution results to user
fn display_execution_results(result: &vortex_namespace::ExecutionResult) {
    println!("\n📊 Execution completed");
    println!("   Exit code: {}", result.exit_code);

    if !result.stdout.is_empty() {
        println!("\n--- STDOUT ---");
        print!("{}", String::from_utf8_lossy(&result.stdout));
    }

    if !result.stderr.is_empty() {
        println!("\n--- STDERR ---");
        eprint!("{}", String::from_utf8_lossy(&result.stderr));
    }
}

/// Stop monitoring and wait for task to complete
async fn stop_monitoring(
    monitor: ResourceMonitor,
    handle: tokio::task::JoinHandle<()>,
) -> Result<()> {
    monitor.stop().await;
    handle.await?;
    Ok(())
}
