//! Namespace executor - executes programs in isolated namespaces

use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult, Pid};
use std::ffi::CString;
use std::os::unix::io::FromRawFd;
use vortex_core::{Error, Result};

use crate::config::NamespaceConfig;
use crate::manager::NamespaceManager;

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Exit code of the command
    pub exit_code: i32,
    /// Standard output captured from the command
    pub stdout: Vec<u8>,
    /// Standard error captured from the command
    pub stderr: Vec<u8>,
}

/// Executor for running programs in isolated namespaces
pub struct NamespaceExecutor {
    config: NamespaceConfig,
}

impl NamespaceExecutor {
    /// Create a new namespace executor
    ///
    /// # Errors
    /// Returns error if namespace creation fails
    pub fn new(config: NamespaceConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Execute a program in the isolated namespace
    ///
    /// This will:
    /// 1. Create pipes for stdout/stderr capture
    /// 2. Fork a new process
    /// 3. In child: Setup namespaces and execute program
    /// 4. In parent: Read output and wait for completion
    ///
    /// # Errors
    /// Returns error if execution fails
    pub fn execute(&self, program: &str, args: &[String]) -> Result<ExecutionResult> {
        tracing::info!(
            program = %program,
            args = ?args,
            "Executing in isolated namespace"
        );

        // Create pipes for stdout and stderr using raw pipe() call
        let stdout_pipe = self.create_pipe()?;
        let stderr_pipe = self.create_pipe()?;

        // Fork process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Parent process
                self.handle_parent(child, stdout_pipe, stderr_pipe)
            }
            Ok(ForkResult::Child) => {
                // Child process - this never returns
                self.handle_child(program, args, stdout_pipe, stderr_pipe);
            }
            Err(e) => Err(Error::Namespace {
                message: format!("Failed to fork: {}", e),
            }),
        }
    }

    /// Create a pipe for IPC using libc directly
    fn create_pipe(&self) -> Result<[i32; 2]> {
        let mut fds = [0i32; 2];
        unsafe {
            if libc::pipe(fds.as_mut_ptr()) == -1 {
                return Err(Error::Namespace {
                    message: format!("Failed to create pipe: {}", std::io::Error::last_os_error()),
                });
            }
        }
        Ok(fds)
    }

    /// Handle parent process after fork
    fn handle_parent(
        &self,
        child: Pid,
        stdout_pipe: [i32; 2],
        stderr_pipe: [i32; 2],
    ) -> Result<ExecutionResult> {
        // Close write ends in parent
        unsafe {
            libc::close(stdout_pipe[1]);
            libc::close(stderr_pipe[1]);
        }

        // Read from pipes
        let stdout = self.read_from_fd(stdout_pipe[0])?;
        let stderr = self.read_from_fd(stderr_pipe[0])?;

        // Close read ends
        unsafe {
            libc::close(stdout_pipe[0]);
            libc::close(stderr_pipe[0]);
        }

        // Wait for child
        let exit_code = self.wait_for_child(child)?;

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Handle child process after fork
    fn handle_child(
        &self,
        program: &str,
        args: &[String],
        stdout_pipe: [i32; 2],
        stderr_pipe: [i32; 2],
    ) -> ! {
        // Close read ends in child
        unsafe {
            libc::close(stdout_pipe[0]);
            libc::close(stderr_pipe[0]);
        }

        // Redirect stdout and stderr using libc directly
        unsafe {
            if libc::dup2(stdout_pipe[1], 1) == -1 {
                eprintln!("Failed to redirect stdout");
                libc::_exit(1);
            }

            if libc::dup2(stderr_pipe[1], 2) == -1 {
                eprintln!("Failed to redirect stderr");
                libc::_exit(1);
            }

            // Close original file descriptors
            libc::close(stdout_pipe[1]);
            libc::close(stderr_pipe[1]);
        }

        // Setup namespaces
        let mut manager = NamespaceManager::new(self.config.clone());
        if let Err(e) = manager.create() {
            eprintln!("Failed to create namespaces: {}", e);
            unsafe {
                libc::_exit(1);
            }
        }

        // Execute program
        self.execute_child(program, args);
    }

    /// Read all data from a file descriptor
    fn read_from_fd(&self, fd: i32) -> Result<Vec<u8>> {
        use std::io::Read;

        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)
            .map_err(|e| Error::Namespace {
                message: format!("Failed to read from pipe: {}", e),
            })?;

        Ok(buffer)
    }

    /// Execute the child program (does not return)
    fn execute_child(&self, program: &str, args: &[String]) -> ! {
        // Convert program and args to C strings
        let program_c = match CString::new(program) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Invalid program path: {}", e);
                unsafe {
                    libc::_exit(1);
                }
            }
        };

        let mut args_c: Vec<CString> = Vec::new();
        args_c.push(program_c.clone()); // First arg is program name

        for arg in args {
            match CString::new(arg.as_str()) {
                Ok(s) => args_c.push(s),
                Err(e) => {
                    eprintln!("Invalid argument: {}", e);
                    unsafe {
                        libc::_exit(1);
                    }
                }
            }
        }

        // Convert to pointers
        let mut args_ptr: Vec<*const libc::c_char> = args_c.iter().map(|s| s.as_ptr()).collect();
        args_ptr.push(std::ptr::null()); // Null-terminated array

        // Execute
        unsafe {
            libc::execvp(program_c.as_ptr(), args_ptr.as_ptr());
        }

        // If we get here, exec failed
        let error = std::io::Error::last_os_error();
        eprintln!("Failed to execute {}: {}", program, error);
        unsafe {
            libc::_exit(127);
        } // Command not found
    }

    /// Wait for child process and get exit code
    fn wait_for_child(&self, child: Pid) -> Result<i32> {
        match waitpid(child, None) {
            Ok(WaitStatus::Exited(_, code)) => {
                tracing::info!(
                    program = "command",
                    exit_code = code,
                    "Command execution completed"
                );
                Ok(code)
            }
            Ok(WaitStatus::Signaled(_, signal, _)) => {
                tracing::warn!(
                    signal = ?signal,
                    "Command terminated by signal"
                );
                Ok(128 + signal as i32)
            }
            Ok(status) => {
                tracing::warn!(
                    status = ?status,
                    "Unexpected wait status"
                );
                Ok(1)
            }
            Err(e) => Err(Error::Namespace {
                message: format!("Failed to wait for child: {}", e),
            }),
        }
    }
}

impl std::fmt::Debug for NamespaceExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NamespaceExecutor")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result_creation() {
        let result = ExecutionResult {
            exit_code: 0,
            stdout: b"hello".to_vec(),
            stderr: vec![],
        };

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, b"hello");
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_execution_result_clone() {
        let result1 = ExecutionResult {
            exit_code: 0,
            stdout: b"test".to_vec(),
            stderr: b"error".to_vec(),
        };

        let result2 = result1.clone();

        assert_eq!(result1.exit_code, result2.exit_code);
        assert_eq!(result1.stdout, result2.stdout);
        assert_eq!(result1.stderr, result2.stderr);
    }

    #[test]
    #[ignore] // Requires root privileges
    fn test_simple_execution() {
        let config = NamespaceConfig::new();
        let executor = NamespaceExecutor::new(config).unwrap();

        let result = executor
            .execute("/bin/echo", &["hello".to_string()])
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(String::from_utf8_lossy(&result.stdout).contains("hello"));
    }

    #[test]
    #[ignore] // Requires root privileges
    fn test_execution_with_stderr() {
        let config = NamespaceConfig::new();
        let executor = NamespaceExecutor::new(config).unwrap();

        // Execute a command that writes to stderr
        let result = executor
            .execute("/bin/sh", &["-c".to_string(), "echo error >&2".to_string()])
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(String::from_utf8_lossy(&result.stderr).contains("error"));
    }

    #[test]
    #[ignore] // Requires root privileges
    fn test_execution_failure() {
        let config = NamespaceConfig::new();
        let executor = NamespaceExecutor::new(config).unwrap();

        // Execute a non-existent command
        let result = executor.execute("/bin/nonexistent", &[]).unwrap();

        // Should return non-zero exit code
        assert_ne!(result.exit_code, 0);
    }
}
