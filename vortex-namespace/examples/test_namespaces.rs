//! Test namespace isolation
//!
//! Run with: sudo cargo run --example test_namespaces

use vortex_namespace::{NamespaceConfig, NamespaceManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    println!("🦀 Vortex Namespace Test\n");

    // Show info before namespace isolation
    println!("📊 Before namespace isolation:");
    show_process_info();
    println!();

    // Create namespace configuration
    let config = NamespaceConfig::new().with_hostname("vortex-test");

    let manager = NamespaceManager::new(config);

    // Enter namespaces
    println!("🔒 Entering namespaces...");
    manager.enter_namespaces()?;
    println!();

    // Show info after namespace isolation
    println!("📊 After namespace isolation:");
    show_process_info();
    println!();

    println!("✅ Namespace test complete!");
    println!("💡 Note: PID namespace takes effect after fork/exec");

    Ok(())
}

fn show_process_info() {
    println!("  PID: {}", std::process::id());

    if let Ok(hostname) = hostname::get() {
        println!("  Hostname: {}", hostname.to_string_lossy());
    }

    if let Ok(cwd) = std::env::current_dir() {
        println!("  CWD: {}", cwd.display());
    }
}
