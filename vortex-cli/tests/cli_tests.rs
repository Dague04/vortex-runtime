use assert_cmd::Command;
use predicates::prelude::*;

/// Check if running as root
fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

#[test]
fn test_help_command() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Lightweight container runtime"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("stop"))
        .stdout(predicate::str::contains("namespaces"))
        .stdout(predicate::str::contains("health"));
}

#[test]
fn test_version_command() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("vortex"));
}

#[test]
fn test_invalid_command() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_run_without_id() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--")
        .arg("/bin/echo")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_run_without_command() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_run_requires_root() {
    // Skip if running as root
    if is_root() {
        return;
    }

    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test")
        .arg("--")
        .arg("/bin/echo")
        .arg("test")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Permission Denied")
                .or(predicate::str::contains("Must run as root")),
        );
}

#[test]
fn test_stats_without_id() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("stats")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_stop_without_id() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("stop")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_list_command() {
    // List command should work (might show empty list or require root)
    let output = Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("list")
        .output()
        .expect("Failed to execute command");

    // Either succeeds (shows containers or "no containers")
    // or fails with permission error
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Containers") || stdout.contains("No containers"),
            "Expected container list output, got: {}",
            stdout
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Permission") || stderr.contains("root"),
            "Expected permission error, got: {}",
            stderr
        );
    }
}

#[test]
fn test_namespaces_no_root_needed() {
    // Namespaces command should work without root
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("namespaces")
        .assert()
        .success()
        .stdout(predicate::str::contains("Namespace"));
}

#[test]
fn test_health_check_without_root() {
    // Skip if running as root
    if is_root() {
        return;
    }

    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("health")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Health Check"))
        .stderr(
            predicate::str::contains("NOT ROOT").or(predicate::str::contains("Must run as root")),
        );
}

#[test]
fn test_run_help() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Run a container"))
        .stdout(predicate::str::contains("--id"))
        .stdout(predicate::str::contains("--cpu"))
        .stdout(predicate::str::contains("--memory"))
        .stdout(predicate::str::contains("--monitor"))
        .stdout(predicate::str::contains("--hostname"));
}

#[test]
fn test_invalid_cpu_value() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test")
        .arg("--cpu")
        .arg("invalid")
        .arg("--")
        .arg("/bin/echo")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_invalid_memory_value() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test")
        .arg("--memory")
        .arg("invalid")
        .arg("--")
        .arg("/bin/echo")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_negative_cpu_value() {
    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test")
        .arg("--cpu")
        .arg("-1")
        .arg("--")
        .arg("/bin/echo")
        .arg("test")
        .assert()
        .failure();
}

#[test]
#[ignore] // Requires root
fn test_command_with_args() {
    // Skip if not root
    if !is_root() {
        return;
    }

    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test-args")
        .arg("--")
        .arg("/bin/echo")
        .arg("hello")
        .arg("world")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world"));
}

#[test]
#[ignore] // Requires root
fn test_monitor_flag() {
    // Skip if not root
    if !is_root() {
        return;
    }

    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test-monitor")
        .arg("--monitor")
        .arg("--")
        .arg("/bin/sleep")
        .arg("1")
        .assert()
        .success();
}

#[test]
#[ignore] // Requires root
fn test_no_namespaces_flag() {
    // Skip if not root
    if !is_root() {
        return;
    }

    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test-no-ns")
        .arg("--no-namespaces")
        .arg("--")
        .arg("/bin/echo")
        .arg("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("Namespaces: disabled"));
}

#[test]
#[ignore] // Requires root
fn test_custom_hostname() {
    // Skip if not root
    if !is_root() {
        return;
    }

    Command::new(env!("CARGO_BIN_EXE_vortex"))
        .arg("run")
        .arg("--id")
        .arg("test-hostname")
        .arg("--hostname")
        .arg("my-test-container")
        .arg("--")
        .arg("/bin/hostname")
        .assert()
        .success()
        .stdout(predicate::str::contains("my-test-container"));
}
