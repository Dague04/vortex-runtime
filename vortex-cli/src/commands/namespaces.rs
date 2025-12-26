//! Namespaces command implementation

use anyhow::{Context, Result};

pub async fn execute(pid: Option<u32>) -> Result<()> {
    let target_pid = pid.unwrap_or_else(|| std::process::id());

    println!("\n🔒 Namespace Information for PID {}", target_pid);
    println!("{:-<60}", "");

    let ns_info = vortex_namespace::NamespaceManager::namespaces_for_pid(target_pid)
        .context("Failed to get namespace information")?;

    print!("{}", ns_info);

    // Check if isolated
    match ns_info.is_isolated() {
        Ok(true) => println!("\n✅ Process is in isolated namespaces"),
        Ok(false) => println!("\n⚠️  Process is in host namespaces"),
        Err(e) => println!("\n❌ Failed to check isolation: {}", e),
    }

    // Read hostname from /proc if UTS namespace is set
    if ns_info.uts.is_some() {
        if let Ok(hostname) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
            println!("Hostname: {}", hostname.trim());
        }
    }

    Ok(())
}
