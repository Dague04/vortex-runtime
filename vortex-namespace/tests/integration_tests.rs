//! Integration tests for namespace management
//! Run with: cargo test -p vortex-namespace
//! Run root tests: sudo cargo test -p vortex-namespace -- --ignored

use vortex_namespace::{NamespaceConfig, NamespaceExecutor, NamespaceManager};

#[test]
fn test_namespace_config_builder() {
    let config = NamespaceConfig::new()
        .with_pid(true)
        .with_network(false)
        .with_hostname("test-container");

    assert!(config.pid);
    assert!(!config.network);
    assert_eq!(config.hostname.as_deref(), Some("test-container"));
}

#[test]
fn test_namespace_manager_default() {
    let manager = NamespaceManager::with_defaults();
    assert!(!manager.is_created());
    assert!(manager.config().has_any());
}

#[test]
fn test_current_namespaces() {
    let manager = NamespaceManager::with_defaults();
    let ns_info = manager
        .current_namespaces()
        .expect("Failed to get namespace info");
    assert!(ns_info.pid.is_some());
}

#[test]
#[ignore]
fn test_namespace_creation_requires_root() {
    if !nix::unistd::getuid().is_root() {
        println!("Skipping - requires root");
        return;
    }

    let config = NamespaceConfig::minimal();
    let mut manager = NamespaceManager::new(config);

    let result = manager.create();
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    assert!(manager.is_created());
}

#[test]
#[ignore]
fn test_executor_command() {
    if !nix::unistd::getuid().is_root() {
        println!("Skipping - requires root");
        return;
    }

    let config = NamespaceConfig::minimal();
    let mut executor = NamespaceExecutor::new(config);

    let result = executor
        .execute("/bin/echo", &["Hello".to_string()])
        .expect("Failed to execute");

    assert!(result.success());
    assert!(result.stdout_string().contains("Hello"));
}
