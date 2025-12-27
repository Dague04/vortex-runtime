use vortex_namespace::*;

#[test]
fn test_namespace_config_creation() {
    let config = NamespaceConfig::new();
    assert!(!config.has_any());
}

#[test]
fn test_namespace_config_minimal() {
    let config = NamespaceConfig::minimal();
    assert!(config.has_any());

    let enabled = config.enabled_namespaces();
    assert!(enabled.contains(&"pid"));
    assert!(enabled.contains(&"mnt"));
    assert!(enabled.contains(&"uts"));
}

#[test]
fn test_namespace_config_with_hostname() {
    let config = NamespaceConfig::minimal().with_hostname("my-container");

    assert_eq!(config.hostname.as_deref(), Some("my-container"));
}

#[test]
fn test_namespace_config_with_domainname() {
    let config = NamespaceConfig::minimal().with_domainname("example.com");

    assert_eq!(config.domainname.as_deref(), Some("example.com"));
}

#[test]
fn test_execution_result() {
    let result = ExecutionResult {
        exit_code: 0,
        stdout: b"hello".to_vec(),
        stderr: Vec::new(),
    };

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, b"hello");
    assert!(result.stderr.is_empty());
}

#[test]
fn test_execution_result_clone() {
    let result1 = ExecutionResult {
        exit_code: 42,
        stdout: b"output".to_vec(),
        stderr: b"error".to_vec(),
    };

    let result2 = result1.clone();

    assert_eq!(result1.exit_code, result2.exit_code);
    assert_eq!(result1.stdout, result2.stdout);
    assert_eq!(result1.stderr, result2.stderr);
}

#[test]
fn test_namespace_executor_creation() {
    let config = NamespaceConfig::new();
    let executor = NamespaceExecutor::new(config);

    assert!(executor.is_ok());
}

#[test]
#[ignore] // Requires root
fn test_namespace_manager_creation() {
    let config = NamespaceConfig::new();
    let manager = NamespaceManager::new(config);

    assert!(!manager.is_created());
}

#[test]
#[ignore] // Requires root
fn test_simple_command_execution() {
    let config = NamespaceConfig::new();
    let executor = NamespaceExecutor::new(config).unwrap();

    let result = executor.execute("/bin/echo", &["hello".to_string()]);

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(String::from_utf8_lossy(&result.stdout).contains("hello"));
}

#[test]
#[ignore] // Requires root
fn test_command_with_stderr() {
    let config = NamespaceConfig::new();
    let executor = NamespaceExecutor::new(config).unwrap();

    let result = executor.execute("/bin/sh", &["-c".to_string(), "echo error >&2".to_string()]);

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(String::from_utf8_lossy(&result.stderr).contains("error"));
}

#[test]
#[ignore] // Requires root
fn test_nonexistent_command() {
    let config = NamespaceConfig::new();
    let executor = NamespaceExecutor::new(config).unwrap();

    let result = executor.execute("/bin/nonexistent", &[]);

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_ne!(result.exit_code, 0);
}
