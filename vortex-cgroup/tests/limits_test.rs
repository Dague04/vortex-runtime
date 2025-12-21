use vortex_cgroup::CGroupController;
use vortex_core::{ContainerId, CpuCores, CpuLimit, MemoryLimit, MemorySize};

#[tokio::test]
async fn test_set_limits() {
    let id = ContainerId::new("test-limits").expect("Valid ID");

    // Try to create cgroup (needs root)
    let cgroup = match CGroupController::new(id).await {
        Ok(c) => c,
        Err(e) => {
            println!("⚠️  Skipping test (need root): {}", e);
            return;
        }
    };

    // Test CPU limit
    let cpu_limit = CpuLimit::new(CpuCores::new(1.5));
    match cgroup.set_cpu_limit(cpu_limit).await {
        Ok(_) => println!("✅ CPU limit set to 1.5 cores"),
        Err(e) => println!("❌ Failed to set CPU limit: {}", e),
    }

    // Test memory limit
    let mem_limit = MemoryLimit::new(MemorySize::from_mb(512)).with_swap(MemorySize::from_mb(1024));

    match cgroup.set_memory_limit(mem_limit).await {
        Ok(_) => println!("✅ Memory limit set to 512MB (swap: 1GB)"),
        Err(e) => println!("❌ Failed to set memory limit: {}", e),
    }
}

#[tokio::test]
async fn test_remove_limits() {
    let id = ContainerId::new("test-remove-limits").expect("Valid ID");

    let cgroup = match CGroupController::new(id).await {
        Ok(c) => c,
        Err(e) => {
            println!("⚠️  Skipping test (need root): {}", e);
            return;
        }
    };

    // Set limits first
    let _ = cgroup
        .set_cpu_limit(CpuLimit::new(CpuCores::new(1.0)))
        .await;
    let _ = cgroup
        .set_memory_limit(MemoryLimit::new(MemorySize::from_mb(256)))
        .await;

    // Now remove them
    match cgroup.remove_cpu_limit().await {
        Ok(_) => println!("✅ CPU limit removed"),
        Err(e) => println!("❌ Failed to remove CPU limit: {}", e),
    }

    match cgroup.remove_memory_limit().await {
        Ok(_) => println!("✅ Memory limit removed"),
        Err(e) => println!("❌ Failed to remove memory limit: {}", e),
    }
}
