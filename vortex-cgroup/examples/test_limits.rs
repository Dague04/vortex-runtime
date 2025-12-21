//! Manual test for CGroup limits
//!
//! Run with: sudo cargo run --example test_limits

use vortex_cgroup::CGroupController;
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize, ProcessId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🦀 Vortex CGroup Limit Test\n");

    // Create cgroup
    let id = ContainerId::new("manual-test")?;
    println!("📦 Creating cgroup: {}", id);
    let cgroup = CGroupController::new(id).await?;
    println!("✅ CGroup created\n");

    // Set CPU limit
    println!("⚙️  Setting CPU limit to 0.5 cores...");
    let cpu_limit = CpuLimit::new(CpuCores::new(0.5));
    cgroup.set_cpu_limit(cpu_limit).await?;
    println!("✅ CPU limit set\n");

    // Set memory limit
    println!("💾 Setting memory limit to 128MB...");
    let mem_limit = MemoryLimit::new(MemorySize::from_mb(128));
    cgroup.set_memory_limit(mem_limit).await?;
    println!("✅ Memory limit set\n");

    // Add current process
    println!("🔗 Adding current process to cgroup...");
    cgroup.add_process(ProcessId::current()).await?;
    println!("✅ Process added\n");

    // Show what we can verify
    println!("📊 Verification:");
    println!("   You can manually check:");
    println!("   cat /sys/fs/cgroup/vortex/manual-test/cpu.max");
    println!("   cat /sys/fs/cgroup/vortex/manual-test/memory.max");
    println!("   cat /sys/fs/cgroup/vortex/manual-test/cgroup.procs");

    println!("\n⏸️  Press Ctrl+C to exit and cleanup...");
    tokio::signal::ctrl_c().await?;

    println!("\n🧹 Cleaning up explicitly...");
    cgroup.cleanup().await?; // ← Explicit async cleanup
    println!("✅ Cleanup complete!");

    Ok(())
}
