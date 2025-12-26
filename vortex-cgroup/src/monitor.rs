//! Resource monitoring system with event emission
//!
//! Provides background monitoring of container resources using Arc<Mutex<T>>
//! for shared access and channels for event emission.

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Duration};
use vortex_core::{ContainerEvent, ContainerId, ResourceStats, Result};

use crate::backend::ResourceBackend;

/// Resource monitor that runs in the background
///
/// # Example
/// ```no_run
/// use std::sync::Arc;
/// use tokio::sync::mpsc;
/// use vortex_cgroup::{MockBackend, ResourceMonitor, ResourceBackend};
/// use vortex_core::ContainerId;
///
/// # tokio_test::block_on(async {
/// let backend = Arc::new(MockBackend::new()) as Arc<dyn ResourceBackend>;
/// let id = ContainerId::new("my-container").unwrap();
/// let (tx, mut rx) = mpsc::channel(100);
///
/// let monitor = ResourceMonitor::new(backend, id, 2)
///     .with_events(tx);
///
/// let handle = monitor.start().await.unwrap();
///
/// // Receive events
/// while let Some(event) = rx.recv().await {
///     println!("Event: {}", event);
/// }
///
/// monitor.stop().await;
/// handle.await.unwrap();
/// # });
/// ```
pub struct ResourceMonitor {
    backend: Arc<dyn ResourceBackend>,
    container_id: ContainerId,
    interval_secs: u64,
    running: Arc<Mutex<bool>>,
    event_tx: Option<mpsc::Sender<ContainerEvent>>,
}

impl ResourceMonitor {
    /// Create a new monitor for a backend
    ///
    /// # Arguments
    /// * `backend` - The resource backend to monitor
    /// * `container_id` - Container identifier
    /// * `interval_secs` - How often to collect stats (in seconds)
    #[must_use]
    pub fn new(
        backend: Arc<dyn ResourceBackend>,
        container_id: ContainerId,
        interval_secs: u64,
    ) -> Self {
        Self {
            backend,
            container_id,
            interval_secs,
            running: Arc::new(Mutex::new(false)),
            event_tx: None,
        }
    }

    /// Add event channel for emitting events
    ///
    /// Events will be sent to this channel as they occur.
    #[must_use]
    pub fn with_events(mut self, tx: mpsc::Sender<ContainerEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Start monitoring in the background
    ///
    /// Returns a join handle that can be awaited to ensure the monitor completes.
    ///
    /// # Errors
    /// Returns error if monitoring cannot be started
    pub async fn start(&self) -> Result<tokio::task::JoinHandle<()>> {
        *self.running.lock().await = true;

        let backend = Arc::clone(&self.backend);
        let running = Arc::clone(&self.running);
        let interval_secs = self.interval_secs;
        let event_tx = self.event_tx.clone();
        let container_id = self.container_id.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            tracing::info!(
                container_id = %container_id,
                interval_secs,
                "Resource monitoring started"
            );

            println!("\n📊 Resource Monitoring Started for {container_id}");
            println!("{:-<80}", "");
            println!(
                "{:<10} {:<15} {:<15} {:<20} {:<20}",
                "Time", "CPU (s)", "Throttled (s)", "Memory", "Peak Memory"
            );
            println!("{:-<80}", "");

            let start = std::time::Instant::now();
            let mut last_stats: Option<ResourceStats> = None;

            // Emit started event
            if let Some(ref tx) = event_tx {
                let event = ContainerEvent::Started {
                    id: container_id.clone(),
                    timestamp: std::time::SystemTime::now(),
                };
                event.emit_trace();
                let _ = tx.send(event).await;
            }

            loop {
                ticker.tick().await;

                // Check if we should stop
                if !*running.lock().await {
                    tracing::debug!("Monitor stopping");
                    break;
                }

                // Read stats
                let stats = backend.stats().await;

                match stats {
                    Ok(s) => {
                        let elapsed = start.elapsed().as_secs();

                        // Check for CPU throttling
                        if let Some(ref prev) = last_stats {
                            let throttle_delta = s.cpu_throttled - prev.cpu_throttled;
                            if throttle_delta > Duration::from_millis(100) {
                                if let Some(ref tx) = event_tx {
                                    let event = ContainerEvent::CpuThrottled {
                                        id: container_id.clone(),
                                        duration: throttle_delta,
                                        timestamp: std::time::SystemTime::now(),
                                    };
                                    event.emit_trace();
                                    let _ = tx.send(event).await;
                                }
                            }

                            // Check for memory pressure (>80%)
                            if s.memory_current.as_bytes() > prev.memory_current.as_bytes() {
                                if let Some(limit) = get_memory_limit(&s) {
                                    let percentage =
                                        (s.memory_current.as_bytes() as f64 / limit as f64) * 100.0;

                                    if percentage > 80.0 {
                                        if let Some(ref tx) = event_tx {
                                            let event = ContainerEvent::MemoryPressure {
                                                id: container_id.clone(),
                                                current: s.memory_current.as_bytes(),
                                                limit,
                                                percentage,
                                                timestamp: std::time::SystemTime::now(),
                                            };
                                            event.emit_trace();
                                            let _ = tx.send(event).await;
                                        }
                                    }
                                }
                            }
                        }

                        // Emit stats update event
                        if let Some(ref tx) = event_tx {
                            let event = ContainerEvent::StatsUpdate {
                                id: container_id.clone(),
                                stats: s.clone(),
                                timestamp: std::time::SystemTime::now(),
                            };
                            let _ = tx.send(event).await;
                        }

                        // Print to console
                        println!(
                            "{:<10} {:<15.2} {:<15.2} {:<20} {:<20}",
                            format!("{elapsed}s"),
                            s.cpu_usage.as_secs_f64(),
                            s.cpu_throttled.as_secs_f64(),
                            s.memory_current,
                            s.memory_peak
                        );

                        last_stats = Some(s);
                    }
                    Err(e) => {
                        if format!("{e}").contains("No such file") {
                            println!("\n✅ Container exited");
                            tracing::info!("Container exited");
                            break;
                        }
                        tracing::error!(error = %e, "Error reading stats");
                        eprintln!("Error reading stats: {e}");
                    }
                }
            }

            tracing::info!(container_id = %container_id, "Monitoring stopped");
        });

        Ok(handle)
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        *self.running.lock().await = false;
        tracing::debug!("Stopping monitor");
    }
}

// Helper to estimate memory limit from stats
fn get_memory_limit(stats: &ResourceStats) -> Option<u64> {
    // If peak is significantly higher than current, use peak as estimate
    if stats.memory_peak > stats.memory_current {
        Some(stats.memory_peak.as_bytes())
    } else {
        None
    }
}

impl std::fmt::Debug for ResourceMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceMonitor")
            .field("container_id", &self.container_id)
            .field("interval_secs", &self.interval_secs)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockBackend;

    #[tokio::test]
    async fn test_monitor_lifecycle() {
        let backend = Arc::new(MockBackend::new()) as Arc<dyn ResourceBackend>;
        let id = ContainerId::new("test").unwrap();
        let monitor = ResourceMonitor::new(backend, id, 1);

        let handle = monitor.start().await.unwrap();

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Stop it
        monitor.stop().await;

        // Wait for completion
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_monitor_with_events() {
        let backend = Arc::new(MockBackend::new()) as Arc<dyn ResourceBackend>;
        let id = ContainerId::new("test").unwrap();
        let (tx, mut rx) = mpsc::channel(100);

        let monitor = ResourceMonitor::new(backend, id, 1).with_events(tx);

        let handle = monitor.start().await.unwrap();

        // Should receive started event
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");

        assert!(matches!(event, ContainerEvent::Started { .. }));

        // Should receive stats update
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("event");

        assert!(matches!(event, ContainerEvent::StatsUpdate { .. }));

        monitor.stop().await;
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_monitor_stop_before_start() {
        let backend = Arc::new(MockBackend::new()) as Arc<dyn ResourceBackend>;
        let id = ContainerId::new("test").unwrap();
        let monitor = ResourceMonitor::new(backend, id, 1);

        // Stop before starting (should not panic)
        monitor.stop().await;
    }
}
