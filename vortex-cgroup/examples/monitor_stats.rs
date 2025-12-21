//! Real-time resource monitoring example
//!
//! Run with: sudo cargo run --example monitor_stats

use std::time::Duration;
use tokio::time::sleep;
use vortex_cgroup::CGroupController;
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize, ProcessId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    println!("🦀 Vortex CGroup Statistics Monitor\n");

    // Create cgroup
    let id = ContainerId::new("monitor-test")?;
    println!("📦 Creating cgroup: {}", id);
    let cgroup = CGroupController::new(id).await?;
    println!("✅ CGroup created\n");

    // Set limits
    println!("⚙️  Setting limits...");
    cgroup
        .set_cpu_limit(CpuLimit::new(CpuCores::new(1.0)))
        .await?;
    cgroup
        .set_memory_limit(MemoryLimit::new(MemorySize::from_mb(256)))
        .await?;
    println!("✅ Limits set\n");

    // Add current process
    println!("🔗 Adding current process to cgroup...");
    cgroup.add_process(ProcessId::current()).await?;
    println!("✅ Process added\n");

    println!("📊 Monitoring statistics (press Ctrl+C to stop)...\n");
    println!(
        "{:>10} | {:>15} | {:>15} | {:>15} | {:>15}",
        "Time", "CPU Usage", "CPU Throttled", "Memory", "I/O R+W"
    );
    println!(
        "{:-<10}-+-{:-<15}-+-{:-<15}-+-{:-<15}-+-{:-<15}",
        "", "", "", "", ""
    );

    let start = std::time::Instant::now();

    // Monitor loop
    loop {
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {
                // Read statistics
                match cgroup.stats().await {
                    Ok(stats) => {
                        let elapsed = start.elapsed().as_secs();

                        println!(
                            "{:>10}s | {:>13.2}ms | {:>13.2}ms | {:>15} | {:>15}",
                            elapsed,
                            stats.cpu_usage.as_secs_f64() * 1000.0,
                            stats.cpu_throttled.as_secs_f64() * 1000.0,
                            stats.memory_current,
                            format!("{:.1}KB", (stats.io_read_bytes + stats.io_write_bytes) as f64 / 1024.0)
                        );
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to read stats: {}", e);
                    }
                }
            }

            _ = tokio::signal::ctrl_c() => {
                println!("\n\n🛑 Stopping monitor...");
                break;
            }
        }
    }

    // Cleanup
    println!("🧹 Cleaning up...");
    cgroup.cleanup().await?;
    println!("✅ Done!");

    Ok(())
}
