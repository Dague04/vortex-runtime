//! Basic CGroup usage example

use vortex_cgroup::{CGroupController, ResourceBackend};
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize, ProcessId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create container ID
    let id = ContainerId::new("example-container")?;

    // Create CGroup controller
    let controller = CGroupController::new(id).await?;

    // Set CPU limit to 1 core
    let cpu_limit = CpuLimit::new(CpuCores::new(1.0));
    controller.set_cpu_limit(cpu_limit).await?;

    // Set memory limit to 512MB
    let memory_limit = MemoryLimit::new(MemorySize::from_mb(512));
    controller.set_memory_limit(memory_limit).await?;

    // Add current process to cgroup
    let current_pid = ProcessId::current();
    controller.add_process(current_pid).await?;

    println!("✅ CGroup configured successfully!");
    println!("   CPU limit: 1.0 cores");
    println!("   Memory limit: 512 MB");
    println!("   Process added: {}", current_pid);

    // Read stats
    let stats = controller.stats().await?;
    println!("\n📊 Current stats:");
    println!("   CPU usage: {:.2}s", stats.cpu_usage.as_secs_f64());
    println!("   Memory: {}", stats.memory_current);
    println!("   Peak memory: {}", stats.memory_peak);

    Ok(())
}
