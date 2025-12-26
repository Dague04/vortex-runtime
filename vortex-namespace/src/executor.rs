//! Namespace executor for running code in isolated namespaces

use std::process::Command;
use vortex_core::{Error, ProcessId, Result};

use crate::config::NamespaceConfig;
use crate::manager::NamespaceManager;

/// Executor for running commands in isolated namespaces
#[derive(Debug)]
pub struct NamespaceExecutor {
    manager: NamespaceManager,
}

impl NamespaceExecutor {
    /// Create a new namespace executor
    #[must_use]
    pub fn new(config: NamespaceConfig) -> Self {
        Self {
            manager: NamespaceManager::new(config),
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(NamespaceConfig::default())
    }

    /// Get the namespace manager
    #[must_use]
    pub fn manager(&self) -> &NamespaceManager {
        &self.manager
    }

    /// Execute a command in isolated namespaces
    ///
    /// # Errors
    /// Returns error if namespace creation or command execution fails
    pub fn execute(&mut self, program: &str, args: &[String]) -> Result<ExecutionResult> {
        tracing::info!(
            program = %program,
            args = ?args,
            "Executing in isolated namespace"
        );

        // Create namespaces
        self.manager.create()?;

        // Get current PID (will be PID 1 in new PID namespace)
        let pid = ProcessId::current();

        tracing::debug!(pid = pid.as_raw(), "Namespaces created, executing command");

        // Execute the command
        let mut cmd = Command::new(program);
        cmd.args(args);

        let output = cmd.output().map_err(|e| {
            tracing::error!(
                program = %program,
                error = %e,
                "Failed to execute command"
            );
            Error::Namespace {
                message: format!("Failed to execute {program}: {e}"),
            }
        })?;

        let exit_code = output.status.code().unwrap_or(-1);

        tracing::info!(
            program = %program,
            exit_code,
            "Command execution completed"
        );

        Ok(ExecutionResult {
            pid,
            exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    /// Execute a closure in isolated namespaces
    ///
    /// # Errors
    /// Returns error if namespace creation fails
    pub fn execute_fn<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        tracing::debug!("Executing function in isolated namespace");

        // Create namespaces
        self.manager.create()?;

        // Execute the function
        f()
    }
}

/// Result of command execution
#[derive(Debug)]
pub struct ExecutionResult {
    /// Process ID
    pub pid: ProcessId,
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: Vec<u8>,
    /// Standard error
    pub stderr: Vec<u8>,
}

impl ExecutionResult {
    /// Check if execution was successful
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get stdout as string
    #[must_use]
    pub fn stdout_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Get stderr as string
    #[must_use]
    pub fn stderr_string(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let config = NamespaceConfig::minimal();
        let executor = NamespaceExecutor::new(config);

        assert!(!executor.manager().is_created());
    }

    #[test]
    fn test_execution_result() {
        let result = ExecutionResult {
            pid: ProcessId::from_raw(123),
            exit_code: 0,
            stdout: b"Hello".to_vec(),
            stderr: Vec::new(),
        };

        assert!(result.success());
        assert_eq!(result.stdout_string(), "Hello");
    }
}
