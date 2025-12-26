//! Resource monitoring example with events

use std::sync::Arc;
use tokio::sync::mpsc;
use vortex_cgroup::{CGroupController, ResourceBackend, ResourceMonitor};
use vortex_core::{ContainerEvent, ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create container
    let id = ContainerId::new("monitored-container")?;
    let controller = CGroupController::new(id.clone()).await?;

    // Set limits
    controller
        .set_cpu_limit(CpuLimit::new(CpuCores::new(0.5)))
        .await?;
    controller
        .set_memory_limit(MemoryLimit::new(MemorySize::from_mb(256)))
        .await?;

    // Wrap in Arc for monitoring
    let backend = Arc::new(controller) as Arc<dyn ResourceBackend>;

    // Create event channel
    let (tx, mut rx) = mpsc::channel(100);

    // Create and start monitor
    let monitor = ResourceMonitor::new(backend, id, 2).with_events(tx);
    let monitor_handle = monitor.start().await?;

    // Spawn event handler
    let event_handle = tokio::spawn(async move {
        println!("\n🎯 Event Handler Started\n");

        while let Some(event) = rx.recv().await {
            match event {
                ContainerEvent::Started { id, .. } => {
                    println!("🚀 Container {} started", id);
                }
                ContainerEvent::CpuThrottled { id, duration, .. } => {
                    println!("⚠️  Container {} CPU throttled for {:?}", id, duration);
                }
                ContainerEvent::MemoryPressure { id, percentage, .. } => {
                    println!("⚠️  Container {} memory at {:.1}%", id, percentage);
                }
                ContainerEvent::StatsUpdate { stats, .. } => {
                    println!(
                        "📊 CPU: {:.2}s, Memory: {}",
                        stats.cpu_usage.as_secs_f64(),
                        stats.memory_current
                    );
                }
                ContainerEvent::Error { id, message, .. } => {
                    println!("❌ Container {} error: {}", id, message);
                }
                _ => {}
            }
        }
    });

    // Let it run for 10 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Stop monitoring
    monitor.stop().await;
    monitor_handle.await?;
    event_handle.await?;

    println!("\n✅ Monitoring completed");

    Ok(())
}
