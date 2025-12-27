//! Interactive namespace demonstration
//!
//! Run with: cargo run --example namespace_demo
//! Run as root: sudo cargo run --example namespace_demo

use vortex_namespace::{NamespaceConfig, NamespaceExecutor, NamespaceManager};

/// Check if running as root
fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

fn main() {
    println!("🔒 Vortex Namespace Demo\n");

    // Initialize tracing
    tracing_subscriber::fmt::init();

    demo_current_namespaces();
    demo_config_options();

    if is_root() {
        println!("\n🔐 Running with root privileges - demonstrating isolation\n");
        demo_namespace_creation();
        demo_command_execution();
    } else {
        println!("\n⚠️  Run with sudo to see namespace isolation demos");
    }
}

fn demo_current_namespaces() {
    println!("📊 Current Process Namespaces:");
    println!("{:-<60}", "");

    let manager = NamespaceManager::with_defaults();
    match manager.current_namespaces() {
        Ok(ns_info) => {
            print!("{}", ns_info);

            match ns_info.is_isolated() {
                Ok(true) => println!("✅ Process is isolated"),
                Ok(false) => println!("⚠️  Process is in host namespaces"),
                Err(e) => println!("❌ Failed to check: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to get namespaces: {}", e),
    }
    println!();
}

fn demo_config_options() {
    println!("⚙️  Configuration Options:\n");

    let configs = vec![
        ("Minimal", NamespaceConfig::minimal()),
        ("All", NamespaceConfig::all()),
        (
            "Custom",
            NamespaceConfig::new()
                .with_pid(true)
                .with_uts(true)
                .with_hostname("demo-container"),
        ),
    ];

    for (name, config) in configs {
        let enabled = config.enabled_namespaces();
        println!("  {}: {}", name, enabled.join(", "));
    }
    println!();
}

fn demo_namespace_creation() {
    println!("🚀 Creating Isolated Namespaces:");
    println!("{:-<60}", "");

    let config = NamespaceConfig::minimal();
    let mut manager = NamespaceManager::new(config);

    match manager.create() {
        Ok(()) => {
            println!("✅ Namespaces created successfully");

            if let Ok(ns_info) = manager.current_namespaces() {
                println!("\n  New namespace IDs:");
                if let Some(pid) = ns_info.pid {
                    println!("    PID: {}", pid);
                }
                if let Some(mnt) = ns_info.mnt {
                    println!("    MNT: {}", mnt);
                }
            }
        }
        Err(e) => println!("❌ Failed to create namespaces: {}", e),
    }
    println!();
}

fn demo_command_execution() {
    println!("🎯 Executing Commands in Namespaces:");
    println!("{:-<60}", "");

    let config = NamespaceConfig::new()
        .with_pid(true)
        .with_uts(true)
        .with_hostname("demo-container");

    let executor = match NamespaceExecutor::new(config) {
        Ok(e) => e,
        Err(e) => {
            println!("❌ Failed to create executor: {}", e);
            return;
        }
    };

    // Test 1: Echo command
    println!("  Test 1: Echo command");
    match executor.execute("/bin/echo", &["Hello from isolated namespace!".to_string()]) {
        Ok(result) => {
            println!("    Exit code: {}", result.exit_code);
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("    Output: {}", stdout.trim());
        }
        Err(e) => println!("    ❌ Failed: {}", e),
    }

    // Test 2: Show hostname
    println!("\n  Test 2: Show hostname");
    match executor.execute("/bin/hostname", &[]) {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("    Hostname: {}", stdout.trim());
        }
        Err(e) => println!("    ❌ Failed: {}", e),
    }

    // Test 3: Show process info
    println!("\n  Test 3: Process info");
    match executor.execute("/bin/sh", &["-c".to_string(), "echo PID: $$".to_string()]) {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("    {}", stdout.trim());
        }
        Err(e) => println!("    ❌ Failed: {}", e),
    }

    println!();
}
