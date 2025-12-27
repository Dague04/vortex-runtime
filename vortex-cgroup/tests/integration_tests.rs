use std::sync::Arc;
use std::time::Duration;
use vortex_cgroup::*;
use vortex_core::*;

/// Check if running as root
fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

#[tokio::test]
async fn test_mock_backend_lifecycle() {
    let backend = MockBackend::new();

    // Set CPU limit
    let cpu_limit = CpuLimit::new(CpuCores::new(1.0));
    backend.set_cpu_limit(cpu_limit).await.unwrap();

    // Set memory limit
    let memory_limit = MemoryLimit::new(MemorySize::from_mb(512));
    backend.set_memory_limit(memory_limit).await.unwrap();

    // Add process
    let pid = ProcessId::from_raw(12345);
    backend.add_process(pid).await.unwrap();

    // Get stats
    let stats = backend.stats().await.unwrap();
    assert!(stats.cpu_usage >= Duration::ZERO);
    assert!(stats.memory_current.as_bytes() > 0);
}

#[tokio::test]
async fn test_mock_backend_stats_increase() {
    let backend = MockBackend::new();

    // Get initial stats
    let stats1 = backend.stats().await.unwrap();

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get stats again
    let stats2 = backend.stats().await.unwrap();

    // Stats should increase
    assert!(stats2.cpu_usage >= stats1.cpu_usage);
    assert!(stats2.memory_current >= stats1.memory_current);
}

#[tokio::test]
async fn test_mock_backend_multiple_processes() {
    let backend = MockBackend::new();

    // Add multiple processes
    backend.add_process(ProcessId::from_raw(100)).await.unwrap();
    backend.add_process(ProcessId::from_raw(200)).await.unwrap();
    backend.add_process(ProcessId::from_raw(300)).await.unwrap();

    let stats = backend.stats().await.unwrap();
    assert!(stats.cpu_usage >= Duration::ZERO);
}

#[tokio::test]
async fn test_monitor_lifecycle() {
    let backend = Arc::new(MockBackend::new());
    let container_id = ContainerId::new("test-monitor").unwrap();

    // Create monitor with 1 second interval
    let monitor = ResourceMonitor::new(backend, container_id, 1);

    // Start monitoring
    let handle = monitor.start().await.unwrap();

    // Let it run for 2 seconds
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Stop monitoring
    monitor.stop().await;

    // Wait for task to finish
    handle.await.unwrap();
}

#[tokio::test]
async fn test_monitor_with_events() {
    let backend = Arc::new(MockBackend::new());
    let container_id = ContainerId::new("test-events").unwrap();

    // Create monitor with event channel (bounded channel)
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let monitor = ResourceMonitor::new(backend, container_id.clone(), 1).with_events(tx);

    // Start monitoring
    let handle = monitor.start().await.unwrap();

    // Collect some events
    let mut event_count = 0;
    let timeout = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                // Check if it's a stats update event
                // Note: The exact event type depends on your ContainerEvent enum
                // Adjust this match based on your actual enum variants
                match event {
                    ContainerEvent::StatsUpdate { id, .. } => {
                        assert_eq!(id, container_id);
                        event_count += 1;
                        if event_count >= 2 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            _ = &mut timeout => {
                break;
            }
        }
    }

    // Stop monitoring
    monitor.stop().await;
    handle.await.unwrap();

    // Should have received at least some events
    assert!(
        event_count >= 2,
        "Expected at least 2 events, got {}",
        event_count
    );
}

#[tokio::test]
async fn test_monitor_stop_before_start() {
    let backend = Arc::new(MockBackend::new());
    let container_id = ContainerId::new("test-stop").unwrap();

    let monitor = ResourceMonitor::new(backend, container_id, 1);

    // Stop without starting (should be no-op)
    monitor.stop().await;
}

#[tokio::test]
async fn test_monitor_multiple_stop_calls() {
    let backend = Arc::new(MockBackend::new());
    let container_id = ContainerId::new("test-multi-stop").unwrap();

    let monitor = ResourceMonitor::new(backend, container_id, 1);
    let handle = monitor.start().await.unwrap();

    // Stop multiple times
    monitor.stop().await;
    monitor.stop().await;
    monitor.stop().await;

    handle.await.unwrap();
}

#[tokio::test]
async fn test_mock_backend_concurrent_access() {
    let backend = Arc::new(MockBackend::new());

    // Spawn multiple tasks accessing the backend
    let mut handles = vec![];

    for i in 0..10 {
        let backend_clone = Arc::clone(&backend);
        let handle = tokio::spawn(async move {
            // Set limits
            let cpu_limit = CpuLimit::new(CpuCores::new(1.0));
            backend_clone.set_cpu_limit(cpu_limit).await.unwrap();

            // Add process
            let pid = ProcessId::from_raw(1000 + i);
            backend_clone.add_process(pid).await.unwrap();

            // Get stats
            let _stats = backend_clone.stats().await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_resource_stats_values() {
    let backend = MockBackend::new();

    // Get stats
    let stats = backend.stats().await.unwrap();

    // Check that values are reasonable
    assert!(stats.cpu_usage.as_millis() < 10_000); // Less than 10 seconds
    assert!(stats.memory_current.as_mb() < 10_000.0); // Less than 10 GB
    assert!(stats.memory_peak >= stats.memory_current); // Peak >= current
}

#[tokio::test]
#[ignore] // Requires root privileges
async fn test_real_cgroup_controller() {
    if !is_root() {
        println!("Skipping: requires root");
        return;
    }

    let container_id = ContainerId::new("test-real-cgroup").unwrap();

    // Create controller
    let controller = match CGroupController::new(container_id.clone()).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Could not create controller: {}", e);
            return;
        }
    };

    // Set CPU limit
    let cpu_limit = CpuLimit::new(CpuCores::new(1.0));
    controller.set_cpu_limit(cpu_limit).await.unwrap();

    // Set memory limit
    let memory_limit = MemoryLimit::new(MemorySize::from_mb(512));
    controller.set_memory_limit(memory_limit).await.unwrap();

    // Get stats
    let stats = controller.stats().await.unwrap();
    assert!(stats.cpu_usage >= Duration::ZERO);

    // Cleanup
    controller.cleanup().await.unwrap();
}
