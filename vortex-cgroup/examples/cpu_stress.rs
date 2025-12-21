//! CPU stress test to see throttling in action
//!
//! Run with: sudo cargo run --example cpu_stress

use std::time::Duration;
use tokio::time::sleep;
use vortex_cgroup::CGroupController;
use vortex_core::{ContainerId, CpuCores, CpuLimit, ProcessId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔥 CPU Stress Test\n");

    // Create cgroup with LOW CPU limit
    let id = ContainerId::new("cpu-stress")?;
    let cgroup = CGroupController::new(id).await?;

    // Limit to 0.2 cores (20% of one CPU)
    println!("⚙️  Setting CPU limit to 0.2 cores (we'll try to use 1.0)...");
    cgroup
        .set_cpu_limit(CpuLimit::new(CpuCores::new(0.2)))
        .await?;

    // Add current process
    cgroup.add_process(ProcessId::current()).await?;
    println!("✅ Process added to cgroup\n");

    println!("🔥 Burning CPU for 5 seconds...");

    // Spawn CPU-intensive work
    let handle = tokio::task::spawn_blocking(|| {
        let start = std::time::Instant::now();
        let mut iterations = 0u64;

        while start.elapsed() < Duration::from_secs(5) {
            // Burn CPU
            for _ in 0..1_000_000 {
                iterations += 1;
            }
        }

        iterations
    });

    // Monitor stats while burning CPU
    for i in 0..5 {
        sleep(Duration::from_secs(1)).await;

        if let Ok(stats) = cgroup.stats().await {
            println!(
                "  {}s: CPU used: {:.2}ms, Throttled: {:.2}ms",
                i + 1,
                stats.cpu_usage.as_secs_f64() * 1000.0,
                stats.cpu_throttled.as_secs_f64() * 1000.0
            );
        }
    }

    let iterations = handle.await?;
    println!("\n✅ Completed {} iterations", iterations);
    println!("💡 You should see throttling increase over time!");

    // Final stats
    if let Ok(stats) = cgroup.stats().await {
        println!("\n📊 Final Statistics:");
        println!(
            "  Total CPU time: {:.2}ms",
            stats.cpu_usage.as_secs_f64() * 1000.0
        );
        println!(
            "  Time throttled: {:.2}ms",
            stats.cpu_throttled.as_secs_f64() * 1000.0
        );
        println!(
            "  Throttle ratio: {:.1}%",
            (stats.cpu_throttled.as_secs_f64() / stats.cpu_usage.as_secs_f64()) * 100.0
        );
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    cgroup.cleanup().await?;

    Ok(())
}
