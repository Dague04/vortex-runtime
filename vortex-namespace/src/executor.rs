//! Process execution in isolated namespaces
//!
//! This module uses `unsafe` for fork() which is inherently unsafe
//! but necessary for proper PID namespace isolation.

#![allow(unsafe_code)]

use nix::mount::{mount, umount, MsFlags};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult, Pid};
use std::ffi::CString;
use tracing::{debug, error, info, warn};
use vortex_core::{Error, Result};

/// Execute a command in an isolated namespace
///
/// This function:
/// 1. Forks the process
/// 2. Child becomes PID 1 in new namespace
/// 3. Child execs the command
/// 4. Parent waits for child
pub fn execute_in_namespace(command: &[String]) -> Result<i32> {
    if command.is_empty() {
        return Err(Error::InvalidConfig("Command cannot be empty".to_string()));
    }

    info!("🚀 Executing command: {}", command.join(" "));

    // Fork the process
    debug!("Forking process...");

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // Parent process
            info!("👨‍👦 Parent process waiting for child (PID {})", child);
            parent_process(child)
        }
        Ok(ForkResult::Child) => {
            // Child process - we're now PID 1 in the namespace!
            // If exec fails, we exit here - never return to Rust runtime
            child_process(command);

            // Should never reach here
            unreachable!("Child process should have exec'd or exited");
        }
        Err(e) => Err(Error::Namespace {
            message: format!("Fork failed: {}", e),
        }),
    }
}

/// Parent process: wait for child and handle signals
fn parent_process(child_pid: Pid) -> Result<i32> {
    debug!("Parent: Setting up signal handler...");

    // Setup signal handler for Ctrl+C
    // Make handler more robust - don't fail if it errors
    let child_pid_for_handler = child_pid;
    if let Err(e) = ctrlc::set_handler(move || {
        warn!("Parent: Received Ctrl+C, forwarding to child...");
        let _ = kill(child_pid_for_handler, Signal::SIGTERM);
    }) {
        warn!("Could not set signal handler: {}", e);
        // Continue anyway - not fatal
    }

    debug!("Parent: Waiting for child to exit...");

    // Wait for child to exit
    loop {
        match waitpid(child_pid, None) {
            Ok(WaitStatus::Exited(_, exit_code)) => {
                info!("👋 Child exited with code: {}", exit_code);
                return Ok(exit_code);
            }
            Ok(WaitStatus::Signaled(_, signal, _)) => {
                warn!("Child terminated by signal: {:?}", signal);
                // Exit codes for signals: 128 + signal number
                return Ok(128 + signal as i32);
            }
            Ok(WaitStatus::Stopped(_, signal)) => {
                debug!("Child stopped by signal: {:?}", signal);
                // Continue waiting
            }
            Ok(status) => {
                debug!("Child status: {:?}", status);
                // Continue waiting for exit
            }
            Err(nix::errno::Errno::EINTR) => {
                // Interrupted by signal, continue waiting
                debug!("Wait interrupted by signal, continuing...");
                continue;
            }
            Err(nix::errno::Errno::ECHILD) => {
                warn!("Child process no longer exists");
                return Ok(0);
            }
            Err(e) => {
                error!("Wait failed: {}", e);
                return Err(Error::Namespace {
                    message: format!("Wait failed: {}", e),
                });
            }
        }
    }
}

/// Set up the container environment
/// This must be called AFTER entering namespaces but BEFORE exec
fn setup_container_environment() -> Result<()> {
    info!("🔧 Setting up container environment...");

    // 1. Remount /proc to reflect new PID namespace
    debug!("   Mounting new /proc");

    // Try to unmount existing /proc
    // Use MS_DETACH to make it safer (lazy unmount)
    match umount("/proc") {
        Ok(_) => debug!("   Unmounted old /proc"),
        Err(e) => debug!("   Could not unmount /proc (continuing anyway): {}", e),
    }

    // Mount new proc filesystem
    // MS_NOSUID | MS_NODEV | MS_NOEXEC for security
    let flags = MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC;

    match mount(Some("proc"), "/proc", Some("proc"), flags, None::<&str>) {
        Ok(_) => {
            debug!("   ✅ New /proc mounted");
        }
        Err(e) => {
            warn!("   Failed to mount /proc: {}", e);
            // Don't fail - /proc might already be OK
        }
    }

    // 2. Change to root directory
    debug!("   Changing to /");
    std::env::set_current_dir("/").map_err(|e| Error::Namespace {
        message: format!("Failed to change directory: {}", e),
    })?;

    // 3. Set environment variables
    debug!("   Setting environment variables");

    unsafe {
        std::env::set_var("HOME", "/root");
        std::env::set_var("PWD", "/");
        std::env::set_var("OLDPWD", "/");
    }

    debug!("   ✅ Container environment ready");

    Ok(())
}

/// Child process: setup and exec command
fn child_process(command: &[String]) -> ! {
    info!("👶 Child process started (we are PID 1 in namespace!)");
    info!("   My PID: {}", std::process::id());

    // Setup container environment (mount /proc, etc.)
    if let Err(e) = setup_container_environment() {
        eprintln!("❌ Failed to setup container environment: {}", e);
        std::process::exit(126);
    }

    // Build the actual command to execute
    let (program, args) = build_command(command);

    info!("   Executing: {} {:?}", program, args);
    // This function never returns - it either execs or exits

    info!("👶 Child process started (we are PID 1 in namespace!)");
    info!("   My PID: {}", std::process::id());

    // Build the actual command to execute
    let (program, args) = build_command(command);

    info!("   Executing: {} {:?}", program, args);

    // Convert to CStrings for exec
    let program_cstring = match CString::new(program.as_bytes()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Invalid program name: {}", e);
            std::process::exit(127);
        }
    };

    // Build args as CStrings (include program name as args[0])
    let mut all_args = vec![program.clone()];
    all_args.extend(args);

    let args_cstrings: Vec<CString> = match all_args
        .iter()
        .map(|arg| CString::new(arg.as_bytes()))
        .collect()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Invalid argument: {}", e);
            std::process::exit(127);
        }
    };

    debug!("Child: Calling execvp...");

    // Exec replaces this process with the command
    // This never returns on success
    let result = nix::unistd::execvp(&program_cstring, &args_cstrings);

    // If we get here, exec failed
    eprintln!("❌ Failed to execute {}: {:?}", program, result);
    std::process::exit(127);
}

/// Build the command with proper arguments
///
/// If command is a shell (/bin/bash, /bin/sh), add -i flag for interactive mode
fn build_command(command: &[String]) -> (String, Vec<String>) {
    if command.is_empty() {
        return ("/bin/sh".to_string(), vec!["-i".to_string()]);
    }

    let program = command[0].clone();
    let mut args = command[1..].to_vec();

    // If it's a shell and no args, make it interactive
    if (program == "/bin/bash" || program == "/bin/sh") && args.is_empty() {
        args.push("-i".to_string());
    }

    (program, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_bash() {
        let cmd = vec!["/bin/bash".to_string()];
        let (prog, args) = build_command(&cmd);
        assert_eq!(prog, "/bin/bash");
        assert_eq!(args, vec!["-i"]);
    }

    #[test]
    fn test_build_command_with_args() {
        let cmd = vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "echo hi".to_string(),
        ];
        let (prog, args) = build_command(&cmd);
        assert_eq!(prog, "/bin/bash");
        assert_eq!(args, vec!["-c", "echo hi"]);
    }

    #[test]
    fn test_build_command_other() {
        let cmd = vec!["echo".to_string(), "hello".to_string()];
        let (prog, args) = build_command(&cmd);
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["hello"]);
    }
}
