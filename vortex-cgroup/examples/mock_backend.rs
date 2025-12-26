//! Mock backend example for testing

use vortex_cgroup::{MockBackend, ResourceBackend};
use vortex_core::{CpuCores, CpuLimit, MemoryLimit, MemorySize, ProcessId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🧪 Testing with MockBackend (no filesystem required)\n");

    // Create mock backend
    let backend = MockBackend::new();

    // Set limits
    backend
        .set_cpu_limit(CpuLimit::new(CpuCores::new(2.0)))
        .await?;
    backend
        .set_memory_limit(MemoryLimit::new(MemorySize::from_mb(1024)))
        .await?;

    println!("✅ Set CPU limit: 2.0 cores");
    println!("✅ Set memory limit: 1024 MB");

    // Add processes
    for i in 100..105 {
        let pid = ProcessId::from_raw(i);
        backend.add_process(pid).await?;
        println!("✅ Added process: {}", pid);
    }

    // Check process
    let check_pid = ProcessId::from_raw(102);
    let has_process = backend.has_process(check_pid).await;
    println!("\n🔍 Has process 102? {}", has_process);

    // Read stats multiple times
    println!("\n📊 Stats over time:");
    for i in 0..5 {
        let stats = backend.stats().await?;
        println!(
            "  Iteration {}: CPU={:.2}s, Memory={}",
            i + 1,
            stats.cpu_usage.as_secs_f64(),
            stats.memory_current
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Check call count
    let calls = backend.call_count().await;
    println!("\n📞 Total backend calls: {}", calls);

    // Cleanup
    backend.cleanup().await?;
    println!("\n✅ Cleanup successful");

    let has_process_after = backend.has_process(check_pid).await;
    println!("🔍 Has process 102 after cleanup? {}", has_process_after);

    Ok(())
}
