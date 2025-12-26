//! Resource backend trait for pluggable implementations

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use vortex_core::{CpuLimit, MemoryLimit, MemorySize, ProcessId, ResourceStats, Result};

/// Trait for resource management backends
///
/// This allows for different implementations:
/// - [`CGroupController`](crate::CGroupController) - Production CGroup v2
/// - [`MockBackend`] - Testing without filesystem
/// - Future: Cloud provider backends
///
/// # Thread Safety
/// All implementations must be `Send + Sync` for use across async tasks.
#[async_trait]
pub trait ResourceBackend: Send + Sync {
    /// Set CPU limit
    ///
    /// # Errors
    /// Returns error if limit cannot be set
    async fn set_cpu_limit(&self, limit: CpuLimit) -> Result<()>;

    /// Set memory limit
    ///
    /// # Errors
    /// Returns error if limit cannot be set
    async fn set_memory_limit(&self, limit: MemoryLimit) -> Result<()>;

    /// Add a process to this resource group
    ///
    /// # Errors
    /// Returns error if process cannot be added
    async fn add_process(&self, pid: ProcessId) -> Result<()>;

    /// Get current resource statistics
    ///
    /// # Errors
    /// Returns error if stats cannot be read
    async fn stats(&self) -> Result<ResourceStats>;

    /// Cleanup resources
    ///
    /// # Errors
    /// Returns error if cleanup fails
    async fn cleanup(&self) -> Result<()>;
}

/// Mock backend for testing (doesn't touch filesystem)
///
/// # Example
/// ```
/// use vortex_cgroup::{MockBackend, ResourceBackend};
/// use vortex_core::{CpuLimit, CpuCores, ProcessId};
///
/// # tokio_test::block_on(async {
/// let backend = MockBackend::new();
///
/// // Set limits
/// backend.set_cpu_limit(CpuLimit::new(CpuCores::new(1.0))).await.unwrap();
///
/// // Add process
/// backend.add_process(ProcessId::from_raw(123)).await.unwrap();
///
/// // Read stats
/// let stats = backend.stats().await.unwrap();
/// assert!(stats.memory_current.as_bytes() > 0);
/// # });
/// ```
#[derive(Clone)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
}

#[derive(Default)]
struct MockState {
    cpu_limit: Option<CpuLimit>,
    memory_limit: Option<MemoryLimit>,
    processes: Vec<ProcessId>,
    stats: ResourceStats,
    call_count: usize,
}

impl MockBackend {
    /// Create a new mock backend
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
        }
    }

    /// Get the number of backend calls made (for testing)
    pub async fn call_count(&self) -> usize {
        self.state.lock().await.call_count
    }

    /// Check if a process has been added
    pub async fn has_process(&self, pid: ProcessId) -> bool {
        self.state.lock().await.processes.contains(&pid)
    }

    /// Set mock stats (for testing)
    pub async fn set_mock_stats(&self, stats: ResourceStats) {
        self.state.lock().await.stats = stats;
    }

    /// Get the current CPU limit (for testing)
    pub async fn cpu_limit(&self) -> Option<CpuLimit> {
        self.state.lock().await.cpu_limit
    }

    /// Get the current memory limit (for testing)
    pub async fn memory_limit(&self) -> Option<MemoryLimit> {
        self.state.lock().await.memory_limit
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MockBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockBackend").finish_non_exhaustive()
    }
}

#[async_trait]
impl ResourceBackend for MockBackend {
    async fn set_cpu_limit(&self, limit: CpuLimit) -> Result<()> {
        let mut state = self.state.lock().await;
        state.cpu_limit = Some(limit);
        state.call_count += 1;

        tracing::debug!(cores = limit.cores.as_f64(), "Mock: Set CPU limit");

        Ok(())
    }

    async fn set_memory_limit(&self, limit: MemoryLimit) -> Result<()> {
        let mut state = self.state.lock().await;
        state.memory_limit = Some(limit);
        state.call_count += 1;

        tracing::debug!(
            limit_mb = limit.limit.as_mb(),
            swap_mb = limit.swap.map(|s| s.as_mb()),
            "Mock: Set memory limit"
        );

        Ok(())
    }

    async fn add_process(&self, pid: ProcessId) -> Result<()> {
        let mut state = self.state.lock().await;

        if !state.processes.contains(&pid) {
            state.processes.push(pid);
        }

        state.call_count += 1;

        tracing::debug!(
            pid = pid.as_raw(),
            total_processes = state.processes.len(),
            "Mock: Added process"
        );

        Ok(())
    }

    async fn stats(&self) -> Result<ResourceStats> {
        let mut state = self.state.lock().await;
        state.call_count += 1;

        // Simulate realistic usage growth
        state.stats.cpu_usage += Duration::from_millis(100);
        state.stats.memory_current =
            MemorySize::from_mb((state.stats.memory_current.as_mb() + 10.0).min(500.0) as u64);

        if state.stats.memory_current > state.stats.memory_peak {
            state.stats.memory_peak = state.stats.memory_current;
        }

        tracing::trace!(
            cpu_secs = state.stats.cpu_usage.as_secs_f64(),
            memory_mb = state.stats.memory_current.as_mb(),
            "Mock: Read stats"
        );

        Ok(state.stats.clone())
    }

    async fn cleanup(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.call_count += 1;

        let process_count = state.processes.len();
        *state = MockState::default();

        tracing::debug!(processes_removed = process_count, "Mock: Cleaned up");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vortex_core::CpuCores;

    #[tokio::test]
    async fn test_mock_backend_lifecycle() {
        let backend = MockBackend::new();

        // Set CPU limit
        let cpu_limit = CpuLimit::new(CpuCores::new(1.5));
        backend.set_cpu_limit(cpu_limit).await.unwrap();
        assert_eq!(backend.call_count().await, 1);
        assert!(backend.cpu_limit().await.is_some());

        // Set memory limit
        let mem_limit = MemoryLimit::new(MemorySize::from_mb(512));
        backend.set_memory_limit(mem_limit).await.unwrap();
        assert_eq!(backend.call_count().await, 2);

        // Add processes
        let pid1 = ProcessId::from_raw(123);
        let pid2 = ProcessId::from_raw(456);

        backend.add_process(pid1).await.unwrap();
        backend.add_process(pid2).await.unwrap();
        assert!(backend.has_process(pid1).await);
        assert!(backend.has_process(pid2).await);
        assert_eq!(backend.call_count().await, 4);

        // Read stats multiple times
        let stats1 = backend.stats().await.unwrap();
        let stats2 = backend.stats().await.unwrap();

        // Stats should grow
        assert!(stats2.cpu_usage > stats1.cpu_usage);
        assert!(stats2.memory_current >= stats1.memory_current);

        // Cleanup
        backend.cleanup().await.unwrap();
        assert!(!backend.has_process(pid1).await);
    }

    #[tokio::test]
    async fn test_mock_backend_stats_growth() {
        let backend = MockBackend::new();

        let mut prev_stats = backend.stats().await.unwrap();

        for _ in 0..5 {
            let stats = backend.stats().await.unwrap();

            // CPU should always increase
            assert!(stats.cpu_usage >= prev_stats.cpu_usage);

            // Memory peak should be monotonic
            assert!(stats.memory_peak >= prev_stats.memory_peak);

            prev_stats = stats;
        }
    }

    #[tokio::test]
    async fn test_mock_backend_duplicate_process() {
        let backend = MockBackend::new();
        let pid = ProcessId::from_raw(123);

        // Add same process twice
        backend.add_process(pid).await.unwrap();
        backend.add_process(pid).await.unwrap();

        // Should only be counted once
        assert!(backend.has_process(pid).await);
    }
}
