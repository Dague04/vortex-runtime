use vortex_core::*;

#[test]
fn test_container_id_validation() {
    // Valid IDs
    assert!(ContainerId::new("test").is_ok());
    assert!(ContainerId::new("test-123").is_ok());
    assert!(ContainerId::new("test_456").is_ok());
    assert!(ContainerId::new("a").is_ok());
    assert!(ContainerId::new("ABC-123_xyz").is_ok());

    // Invalid IDs - empty
    assert!(ContainerId::new("").is_err());

    // Invalid IDs - too long
    assert!(ContainerId::new("a".repeat(65)).is_err());

    // Invalid IDs - bad characters
    assert!(ContainerId::new("test@123").is_err());
    assert!(ContainerId::new("test space").is_err());
    assert!(ContainerId::new("test/path").is_err());
    assert!(ContainerId::new("test\\path").is_err());
    assert!(ContainerId::new("test:colon").is_err());
    assert!(ContainerId::new("test;semicolon").is_err());
    assert!(ContainerId::new("test.dot").is_err());
}

#[test]
fn test_container_id_serialization() {
    let id = ContainerId::new("test-123").unwrap();

    // Serialize to JSON
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"test-123\"");

    // Deserialize from JSON
    let deserialized: ContainerId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
}

#[test]
fn test_container_id_display() {
    let id = ContainerId::new("my-container").unwrap();
    assert_eq!(format!("{}", id), "my-container");
    assert_eq!(id.as_str(), "my-container");
}

#[test]
fn test_memory_size_conversions() {
    let size = MemorySize::from_mb(512);

    // Check byte conversion
    assert_eq!(size.as_bytes(), 536_870_912);

    // Check other unit conversions
    assert_eq!(size.as_kb(), 524_288.0);
    assert_eq!(size.as_mb(), 512.0);
    assert_eq!(size.as_gb(), 0.5);
}

#[test]
fn test_memory_size_from_different_units() {
    // From bytes
    let from_bytes = MemorySize::from_bytes(1_073_741_824);
    assert_eq!(from_bytes.as_gb(), 1.0);

    // From KB
    let from_kb = MemorySize::from_kb(1_048_576);
    assert_eq!(from_kb.as_gb(), 1.0);

    // From MB
    let from_mb = MemorySize::from_mb(1024);
    assert_eq!(from_mb.as_gb(), 1.0);

    // From GB
    let from_gb = MemorySize::from_gb(1);
    assert_eq!(from_gb.as_bytes(), 1_073_741_824);
}

#[test]
fn test_memory_size_arithmetic() {
    let a = MemorySize::from_mb(256);
    let b = MemorySize::from_mb(256);
    let c = a + b;

    assert_eq!(c.as_mb(), 512.0);

    let d = MemorySize::from_mb(1024);
    let e = MemorySize::from_mb(512);
    let f = d - e;

    assert_eq!(f.as_mb(), 512.0);
}

#[test]
fn test_memory_size_display() {
    // Memory size should have Display trait
    assert_eq!(format!("{}", MemorySize::from_bytes(1024)), "1.00 KB");
    assert_eq!(format!("{}", MemorySize::from_kb(1024)), "1.00 MB");
    assert_eq!(format!("{}", MemorySize::from_mb(1024)), "1.00 GB");
    assert_eq!(format!("{}", MemorySize::from_mb(512)), "512.00 MB");
}

#[test]
fn test_cpu_cores_quota() {
    // 1.0 cores
    let cores = CpuCores::new(1.0);
    let (quota, period) = cores.to_quota();
    assert_eq!(quota, 100_000);
    assert_eq!(period, 100_000);

    // 1.5 cores
    let cores = CpuCores::new(1.5);
    let (quota, period) = cores.to_quota();
    assert_eq!(quota, 150_000);
    assert_eq!(period, 100_000);

    // 0.5 cores
    let cores = CpuCores::new(0.5);
    let (quota, period) = cores.to_quota();
    assert_eq!(quota, 50_000);
    assert_eq!(period, 100_000);

    // 2.0 cores
    let cores = CpuCores::new(2.0);
    let (quota, period) = cores.to_quota();
    assert_eq!(quota, 200_000);
    assert_eq!(period, 100_000);
}

#[test]
fn test_cpu_cores_as_f64() {
    // Test the as_f64 method
    let cores = CpuCores::new(1.0);
    assert_eq!(cores.as_f64(), 1.0);

    let cores = CpuCores::new(1.5);
    assert_eq!(cores.as_f64(), 1.5);

    let cores = CpuCores::new(0.5);
    assert_eq!(cores.as_f64(), 0.5);
}

#[test]
fn test_process_id() {
    let pid = ProcessId::from_raw(1234);
    assert_eq!(pid.as_raw(), 1234);

    let current = ProcessId::current();
    assert!(current.as_raw() > 0);
}

#[test]
fn test_cpu_limit() {
    let limit = CpuLimit::new(CpuCores::new(1.5));
    assert_eq!(limit.cores.as_f64(), 1.5);
}

#[test]
fn test_memory_limit() {
    let limit = MemoryLimit::new(MemorySize::from_mb(512));
    assert_eq!(limit.limit.as_mb(), 512.0);
    assert!(limit.swap.is_none());

    let limit_with_swap =
        MemoryLimit::with_swap(MemorySize::from_mb(512), MemorySize::from_mb(256));
    assert_eq!(limit_with_swap.limit.as_mb(), 512.0);
    assert_eq!(limit_with_swap.swap.unwrap().as_mb(), 256.0);
}

#[test]
fn test_resource_stats_default() {
    let stats = ResourceStats::default();
    assert_eq!(stats.cpu_usage.as_secs(), 0);
    assert_eq!(stats.cpu_throttled.as_secs(), 0);
    assert_eq!(stats.memory_current.as_bytes(), 0);
    assert_eq!(stats.memory_peak.as_bytes(), 0);
}

#[test]
fn test_container_event_serialization() {
    use std::time::SystemTime;

    let event = ContainerEvent::Started {
        id: ContainerId::new("test").unwrap(),
        timestamp: SystemTime::UNIX_EPOCH,
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: ContainerEvent = serde_json::from_str(&json).unwrap();

    match deserialized {
        ContainerEvent::Started { id, .. } => {
            assert_eq!(id.as_str(), "test");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_memory_size_comparison() {
    let small = MemorySize::from_mb(256);
    let large = MemorySize::from_mb(512);

    assert!(small < large);
    assert!(large > small);
    assert_eq!(small, MemorySize::from_mb(256));
}

#[test]
fn test_container_id_clone() {
    let id1 = ContainerId::new("test").unwrap();
    let id2 = id1.clone();

    assert_eq!(id1, id2);
    assert_eq!(id1.as_str(), id2.as_str());
}

#[test]
fn test_memory_size_zero() {
    let zero = MemorySize::from_bytes(0);
    assert_eq!(zero.as_bytes(), 0);
    assert_eq!(zero.as_mb(), 0.0);
}

#[test]
fn test_cpu_cores_zero() {
    let zero = CpuCores::new(0.0);
    assert_eq!(zero.as_f64(), 0.0);

    let (quota, period) = zero.to_quota();
    assert_eq!(quota, 0);
    assert_eq!(period, 100_000);
}
