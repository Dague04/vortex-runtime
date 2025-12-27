use anyhow::Result;
use std::path::Path;

/// Execute health check command
pub async fn execute() -> Result<()> {
    println!("\n🏥 Vortex Health Check\n");
    println!("{:-<60}", "");

    // Check 1: CGroup v2
    check_cgroup_v2()?;

    // Check 2: Permissions
    check_permissions()?;

    // Check 3: Namespace support
    check_namespace_support()?;

    // Check 4: Required binaries
    check_binaries()?;

    println!("{:-<60}", "");
    println!("\n✅ All systems operational!\n");

    Ok(())
}

/// Check if running as root
fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

/// Check if CGroup v2 is available and properly configured
fn check_cgroup_v2() -> Result<()> {
    print!("Checking CGroup v2... ");

    let cgroup_root = Path::new("/sys/fs/cgroup");

    if !cgroup_root.exists() {
        println!("❌ NOT FOUND");
        anyhow::bail!(
            "CGroup v2 not mounted at /sys/fs/cgroup\n\
             \n\
             To check your CGroup configuration:\n\
             $ mount | grep cgroup2\n\
             \n\
             CGroup v2 is required for Vortex to function."
        );
    }

    // Check if it's actually cgroup v2 (not v1)
    let cgroup_controllers = cgroup_root.join("cgroup.controllers");
    if !cgroup_controllers.exists() {
        println!("❌ CGROUP v1 DETECTED");
        anyhow::bail!(
            "CGroup v1 detected, but Vortex requires CGroup v2\n\
             \n\
             You may need to:\n\
             • Update your kernel (5.0+)\n\
             • Change kernel boot parameters\n\
             • Disable CGroup v1 in systemd"
        );
    }

    // Check available controllers
    match std::fs::read_to_string(&cgroup_controllers) {
        Ok(controllers) => {
            let has_cpu = controllers.contains("cpu");
            let has_memory = controllers.contains("memory");
            let has_io = controllers.contains("io");

            if !has_cpu || !has_memory || !has_io {
                println!("⚠️  INCOMPLETE");
                println!("   Available: {}", controllers.trim());
                println!("   Missing: {}", {
                    let mut missing = Vec::new();
                    if !has_cpu {
                        missing.push("cpu");
                    }
                    if !has_memory {
                        missing.push("memory");
                    }
                    if !has_io {
                        missing.push("io");
                    }
                    missing.join(", ")
                });
                anyhow::bail!(
                    "Required controllers not available\n\
                     \n\
                     Vortex requires: cpu, memory, io"
                );
            }

            println!("✅ OK (cpu, memory, io available)");
        }
        Err(e) => {
            println!("❌ ERROR");
            anyhow::bail!("Could not read controllers: {}", e);
        }
    }

    Ok(())
}

/// Check if running with proper permissions
fn check_permissions() -> Result<()> {
    print!("Checking permissions... ");

    if !is_root() {
        println!("❌ NOT ROOT");
        anyhow::bail!(
            "Must run as root\n\
             \n\
             Vortex requires root permissions to:\n\
             • Create and manage cgroups\n\
             • Create namespaces\n\
             • Access /sys/fs/cgroup\n\
             \n\
             Try: sudo vortex health"
        );
    }

    // Check if we can write to cgroup root
    let test_dir = Path::new("/sys/fs/cgroup/vortex-health-test");
    match std::fs::create_dir(test_dir) {
        Ok(()) => {
            // Clean up test directory
            let _ = std::fs::remove_dir(test_dir);
            println!("✅ OK (root with write access)");
        }
        Err(e) => {
            println!("⚠️  LIMITED");
            println!("   Cannot write to /sys/fs/cgroup: {}", e);
            println!("   This may cause issues");
        }
    }

    Ok(())
}

/// Check if namespace support is available
fn check_namespace_support() -> Result<()> {
    print!("Checking namespace support... ");

    // Check /proc/self/ns/ exists
    let ns_dir = Path::new("/proc/self/ns");
    if !ns_dir.exists() {
        println!("❌ NOT SUPPORTED");
        anyhow::bail!(
            "Kernel doesn't support namespaces\n\
             \n\
             Your kernel may be too old or compiled without namespace support."
        );
    }

    // Check for required namespace types
    let required = ["pid", "mnt", "uts", "ipc", "net"];
    let mut missing = Vec::new();

    for ns_type in &required {
        let ns_file = ns_dir.join(ns_type);
        if !ns_file.exists() {
            missing.push(*ns_type);
        }
    }

    if !missing.is_empty() {
        println!("❌ INCOMPLETE");
        println!("   Missing: {}", missing.join(", "));
        anyhow::bail!(
            "Required namespace types not available\n\
             \n\
             Your kernel may need to be reconfigured."
        );
    }

    println!("✅ OK (all types available)");
    Ok(())
}

/// Check if required binaries are available
fn check_binaries() -> Result<()> {
    print!("Checking required binaries... ");

    let required = ["/bin/sh", "/bin/bash"];
    let mut missing = Vec::new();

    for binary in &required {
        let path = Path::new(binary);
        if !path.exists() {
            missing.push(*binary);
        }
    }

    if !missing.is_empty() {
        println!("⚠️  MISSING");
        println!("   Not found: {}", missing.join(", "));
        println!("   Containers may not work properly");
    } else {
        println!("✅ OK");
    }

    Ok(())
}
