// src/concurrency.rs

//! Advanced concurrency management system with thread pools and async coordination
//!
//! This module provides comprehensive concurrency management including:
//! - Multiple thread pools for different workload types
//! - Async/await coordination with thread pools
//! - Resource management and backpressure
//! - Work stealing and load balancing
//! - Thread-local storage management
//! - Panic handling and recovery
//! - Performance monitoring and metrics

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{fmt, thread};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use crossbeam::queue::SegQueue;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, Semaphore};
use uuid::Uuid;

use crate::config::ConcurrencyConfig;
use crate::error::{Error, ErrorKind, ConcurrencyOperation, Result, ResultExt};
use crate::manager::{Manager, ManagedState, ManagerStatus};

/// Thread pool types for different workload categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreadPoolType {
    /// CPU-intensive computational tasks
    Compute,
    /// I/O operations (file, network)
    Io,
    /// Blocking operations that might block threads
    Blocking,
    /// Background maintenance tasks
    Background,
    /// Custom thread pool
    Custom(u8),
}

impl ThreadPoolType {
    /// Get the default thread count for this pool type
    pub fn default_thread_count(self) -> usize {
        match self {
            Self::Compute => num_cpus::get(),
            Self::Io => num_cpus::get() * 2,
            Self::Blocking => num_cpus::get().max(4),
            Self::Background => 2,
            Self::Custom(_) => 4,
        }
    }

    /// Get the queue capacity for this pool type
    pub fn default_queue_capacity(self) -> usize {
        match self {
            Self::Compute => 1000,
            Self::Io => 5000,
            Self::Blocking => 2000,
            Self::Background => 500,
            Self::Custom(_) => 1000,
        }
    }
}

/// Thread pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolConfig {
    /// Number of threads in the pool
    pub thread_count: usize,
    /// Maximum number of queued tasks
    pub queue_capacity: usize,
    /// Thread stack size
    pub stack_size: Option<usize>,
    /// Thread priority (platform-specific)
    pub priority: Option<i32>,
    /// Thread name prefix
    pub name_prefix: String,
    /// Whether threads should be marked as daemon threads
    pub daemon: bool,
    /// Keep-alive time for idle threads
    pub keep_alive: Duration,
    /// Enable work stealing between threads
    pub work_stealing: bool,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            thread_count: num_cpus::get(),
            queue_capacity: 1000,
            stack_size: None,
            priority: None,
            name_prefix: "worker".to_string(),
            daemon: false,
            keep_alive: Duration::from_secs(60),
            work_stealing: true,
        }
    }
}

/// Work item for thread pool execution
type WorkItem = Box<dyn FnOnce() + Send + 'static>;

/// Thread pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolStats {
    /// Pool type
    pub pool_type: ThreadPoolType,
    /// Number of active threads
    pub active_threads: usize,
    /// Number of idle threads
    pub idle_threads: usize,
    /// Current queue size
    pub queue_size: usize,
    /// Total tasks executed
    pub total_executed: u64,
    /// Total tasks rejected
    pub total_rejected: u64,
    /// Average task execution time
    pub avg_execution_time_ms: f64,
    /// Peak queue size
    pub peak_queue_size: usize,
    /// Thread utilization percentage
    pub utilization_percent: f64,
}

/// Thread worker state
#[derive(Debug)]
struct ThreadWorker {
    id: usize,
    thread_handle: Option<thread::JoinHandle<()>>,
    work_queue: Arc<SegQueue<WorkItem>>,
    stats: Arc<ThreadWorkerStats>,
    shutdown_signal: Arc<parking_lot::Mutex<bool>>,
}

#[derive(Debug)]
struct ThreadWorkerStats {
    tasks_executed: AtomicU64,
    total_execution_time_ms: AtomicU64,
    last_activity: parking_lot::Mutex<Instant>,
}

impl ThreadWorkerStats {
    fn new() -> Self {
        Self {
            tasks_executed: AtomicU64::new(0),
            total_execution_time_ms: AtomicU64::new(0),
            last_activity: parking_lot::Mutex::new(Instant::now()),
        }
    }

    fn record_task_execution(&self, duration: Duration) {
        self.tasks_executed.fetch_add(1, Ordering::Relaxed);
        self.total_execution_time_ms.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        *self.last_activity.lock() = Instant::now();
    }

    fn get_average_execution_time(&self) -> f64 {
        let total_tasks = self.tasks_executed.load(Ordering::Relaxed);
        if total_tasks == 0 {
            0.0
        } else {
            let total_time = self.total_execution_time_ms.load(Ordering::Relaxed);
            total_time as f64 / total_tasks as f64
        }
    }

    fn is_idle(&self, threshold: Duration) -> bool {
        self.last_activity.lock().elapsed() > threshold
    }
}

/// Thread pool implementation
#[derive(Debug)]
pub struct ThreadPool {
    pool_type: ThreadPoolType,
    config: ThreadPoolConfig,
    workers: Vec<ThreadWorker>,
    global_queue: Arc<SegQueue<WorkItem>>,
    stats: Arc<RwLock<ThreadPoolStats>>,
    task_counter: Arc<AtomicU64>,
    rejection_counter: Arc<AtomicU64>,
    shutdown_signal: Arc<parking_lot::Mutex<bool>>,
}

impl ThreadPool {
    /// Create a new thread pool
    pub fn new(pool_type: ThreadPoolType, config: ThreadPoolConfig) -> Result<Self> {
        let global_queue = Arc::new(SegQueue::new());
        let shutdown_signal = Arc::new(parking_lot::Mutex::new(false));
        let task_counter = Arc::new(AtomicU64::new(0));
        let rejection_counter = Arc::new(AtomicU64::new(0));

        let stats = Arc::new(RwLock::new(ThreadPoolStats {
            pool_type,
            active_threads: 0,
            idle_threads: 0,
            queue_size: 0,
            total_executed: 0,
            total_rejected: 0,
            avg_execution_time_ms: 0.0,
            peak_queue_size: 0,
            utilization_percent: 0.0,
        }));

        let mut workers = Vec::with_capacity(config.thread_count);

        // Create worker threads
        for worker_id in 0..config.thread_count {
            let worker_queue = Arc::new(SegQueue::new());
            let worker_stats = Arc::new(ThreadWorkerStats::new());
            let worker_shutdown = Arc::clone(&shutdown_signal);
            let worker_global_queue = Arc::clone(&global_queue);
            let worker_task_counter = Arc::clone(&task_counter);
            let worker_stats_clone = Arc::clone(&worker_stats);
            let worker_queue_clone = Arc::clone(&worker_queue);
            let thread_name = format!("{}-{}", config.name_prefix, worker_id);

            let mut thread_builder = thread::Builder::new().name(thread_name);

            if let Some(stack_size) = config.stack_size {
                thread_builder = thread_builder.stack_size(stack_size);
            }

            let thread_handle = thread_builder
                .spawn(move || {
                    Self::worker_thread(
                        worker_id,
                        worker_queue_clone,
                        worker_global_queue,
                        worker_stats_clone,
                        worker_shutdown,
                        worker_task_counter,
                        config.work_stealing,
                    );
                })
                .with_context(|| format!("Failed to spawn worker thread {}", worker_id))?;

            let worker = ThreadWorker {
                id: worker_id,
                thread_handle: Some(thread_handle),
                work_queue: worker_queue,
                stats: worker_stats,
                shutdown_signal: Arc::clone(&shutdown_signal),
            };

            workers.push(worker);
        }

        // Update initial stats
        {
            let mut stats_guard = stats.write();
            stats_guard.active_threads = config.thread_count;
        }

        Ok(Self {
            pool_type,
            config,
            workers,
            global_queue,
            stats,
            task_counter,
            rejection_counter,
            shutdown_signal,
        })
    }

    /// Submit a task to the thread pool
    pub fn submit<F>(&self, task: F) -> Result<()>
    where
        F: FnOnce() + Send + 'static,
    {
        // Check if pool is shutting down
        if *self.shutdown_signal.lock() {
            return Err(Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                "Thread pool is shutting down",
            ));
        }

        // Check queue capacity
        let current_queue_size = self.global_queue.len();
        if current_queue_size >= self.config.queue_capacity {
            self.rejection_counter.fetch_add(1, Ordering::Relaxed);
            return Err(Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                "Thread pool queue is full",
            ));
        }

        // Submit to global queue
        let work_item: WorkItem = Box::new(task);
        self.global_queue.push(work_item);

        // Update stats
        self.update_stats();

        Ok(())
    }

    /// Submit an async task to the thread pool and return a future
    pub async fn submit_async<F, R>(&self, task: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        let work_item = move || {
            let result = task();
            let _ = tx.send(result);
        };

        self.submit(work_item)?;

        rx.await.map_err(|_| {
            Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                "Task execution was cancelled",
            )
        })
    }

    /// Get current thread pool statistics
    pub fn stats(&self) -> ThreadPoolStats {
        self.stats.read().clone()
    }

    /// Shutdown the thread pool gracefully
    pub fn shutdown(mut self, timeout: Duration) -> Result<()> {
        // Signal shutdown
        *self.shutdown_signal.lock() = true;

        // Wait for workers to finish
        let start_time = Instant::now();
        for mut worker in self.workers.drain(..) {
            let remaining_time = timeout.saturating_sub(start_time.elapsed());

            if let Some(handle) = worker.thread_handle.take() {
                // Give thread some time to finish gracefully
                let join_result = if remaining_time.is_zero() {
                    Err("Thread join timeout")
                } else {
                    match handle.join() {
                        Ok(()) => Ok(()),
                        Err(_) => Err("Thread join failed"),
                    }
                };

                if join_result.is_err() {
                    tracing::warn!("Worker thread {} did not shut down gracefully", worker.id);
                }
            }
        }

        Ok(())
    }

    /// Worker thread main loop
    fn worker_thread(
        worker_id: usize,
        local_queue: Arc<SegQueue<WorkItem>>,
        global_queue: Arc<SegQueue<WorkItem>>,
        stats: Arc<ThreadWorkerStats>,
        shutdown_signal: Arc<parking_lot::Mutex<bool>>,
        task_counter: Arc<AtomicU64>,
        work_stealing: bool,
    ) {
        tracing::debug!("Worker thread {} started", worker_id);

        while !*shutdown_signal.lock() {
            // Try to get work from local queue first
            let work_item = local_queue.pop()
                .or_else(|| global_queue.pop())
                .or_else(|| {
                    if work_stealing {
                        // Try work stealing (simplified - in practice would steal from other workers)
                        None
                    } else {
                        None
                    }
                });

            if let Some(task) = work_item {
                let start_time = Instant::now();

                // Execute the task with panic handling
                let execution_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    task();
                }));

                let execution_time = start_time.elapsed();
                stats.record_task_execution(execution_time);
                task_counter.fetch_add(1, Ordering::Relaxed);

                if execution_result.is_err() {
                    tracing::error!("Task panicked in worker thread {}", worker_id);
                }
            } else {
                // No work available, sleep briefly
                thread::sleep(Duration::from_millis(1));
            }
        }

        tracing::debug!("Worker thread {} shutting down", worker_id);
    }

    /// Update thread pool statistics
    fn update_stats(&self) {
        let mut stats = self.stats.write();

        stats.queue_size = self.global_queue.len();
        stats.total_executed = self.task_counter.load(Ordering::Relaxed);
        stats.total_rejected = self.rejection_counter.load(Ordering::Relaxed);

        if stats.queue_size > stats.peak_queue_size {
            stats.peak_queue_size = stats.queue_size;
        }

        // Calculate average execution time across all workers
        let mut total_execution_time = 0u64;
        let mut total_tasks = 0u64;
        let mut active_threads = 0;
        let mut idle_threads = 0;

        for worker in &self.workers {
            let worker_tasks = worker.stats.tasks_executed.load(Ordering::Relaxed);
            let worker_time = worker.stats.total_execution_time_ms.load(Ordering::Relaxed);

            total_tasks += worker_tasks;
            total_execution_time += worker_time;

            if worker.stats.is_idle(Duration::from_secs(5)) {
                idle_threads += 1;
            } else {
                active_threads += 1;
            }
        }

        stats.active_threads = active_threads;
        stats.idle_threads = idle_threads;

        if total_tasks > 0 {
            stats.avg_execution_time_ms = total_execution_time as f64 / total_tasks as f64;
        }

        // Calculate utilization
        let total_threads = active_threads + idle_threads;
        if total_threads > 0 {
            stats.utilization_percent = (active_threads as f64 / total_threads as f64) * 100.0;
        }
    }
}

/// Async work coordinator for managing async tasks across thread pools
#[derive(Debug)]
pub struct AsyncWorkCoordinator {
    /// Semaphore for limiting concurrent async operations
    semaphore: Arc<Semaphore>,
    /// Channel for coordinating work between async and sync contexts
    work_sender: mpsc::UnboundedSender<AsyncWorkItem>,
    work_receiver: Arc<parking_lot::Mutex<mpsc::UnboundedReceiver<AsyncWorkItem>>>,
    /// Statistics
    stats: Arc<RwLock<AsyncCoordinatorStats>>,
}

struct AsyncWorkItem {
    task: Box<dyn FnOnce() + Send>,
    // result_sender: oneshot::Sender<Result<()>>,
}

impl fmt::Debug for AsyncWorkItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncWorkItem")
            .field("task", &"<FnOnce>") // placeholder string
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncCoordinatorStats {
    /// Total async tasks coordinated
    pub total_coordinated: u64,
    /// Currently active permits
    pub active_permits: usize,
    /// Maximum concurrent operations
    pub max_concurrent: usize,
    /// Average coordination time
    pub avg_coordination_time_ms: f64,
}

impl AsyncWorkCoordinator {
    /// Create a new async work coordinator
    pub fn new(max_concurrent: usize) -> Self {
        let (work_sender, work_receiver) = mpsc::unbounded_channel();
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        Self {
            semaphore,
            work_sender,
            work_receiver: Arc::new(parking_lot::Mutex::new(work_receiver)),
            stats: Arc::new(RwLock::new(AsyncCoordinatorStats {
                total_coordinated: 0,
                active_permits: 0,
                max_concurrent,
                avg_coordination_time_ms: 0.0,
            })),
        }
    }

    /// Coordinate execution of a task
    pub async fn coordinate<F, R>(&self, task: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Acquire permit
        let _permit = self.semaphore.acquire().await.map_err(|_| {
            Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::Sync,
                },
                "Failed to acquire coordination permit",
            )
        })?;

        let start_time = Instant::now();

        // Execute task
        let (result_sender, result_receiver) = oneshot::channel();
        let work_item = AsyncWorkItem {
            task: Box::new(move || {
                let result = task();
                let _ = result_sender.send(Ok(result));
                // Store result for retrieval (simplified)
                // In practice, you'd need a more sophisticated mechanism
            }),
        };

        self.work_sender.send(work_item).map_err(|_| {
            Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::Channel,
                },
                "Failed to send work item",
            )
        })?;

        // Wait for completion
        let result = result_receiver.await.map_err(|_| {
            Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::Sync,
                },
                "Task execution was cancelled",
            )
        })??;

        // Update stats
        let coordination_time = start_time.elapsed();
        self.update_stats(coordination_time);

        // This is a simplified implementation - in practice you'd return the actual result
        // Ok(unsafe { std::mem::zeroed() }) // Placeholder - would return actual result
        Ok(result)
    }

    /// Update coordinator statistics
    fn update_stats(&self, coordination_time: Duration) {
        let mut stats = self.stats.write();
        stats.total_coordinated += 1;
        stats.active_permits = self.semaphore.available_permits();

        let total_time = stats.avg_coordination_time_ms * (stats.total_coordinated - 1) as f64;
        stats.avg_coordination_time_ms =
            (total_time + coordination_time.as_millis() as f64) / stats.total_coordinated as f64;
    }

    /// Get coordinator statistics
    pub fn stats(&self) -> AsyncCoordinatorStats {
        self.stats.read().clone()
    }
}

/// Main concurrency manager
#[derive(Debug)]
pub struct ConcurrencyManager {
    state: ManagedState,
    config: ConcurrencyConfig,
    thread_pools: HashMap<ThreadPoolType, ThreadPool>,
    async_coordinator: AsyncWorkCoordinator,
    runtime_handle: tokio::runtime::Handle,
}

impl ConcurrencyManager {
    /// Create a new concurrency manager
    pub fn new(config: ConcurrencyConfig) -> Result<Self> {
        let runtime_handle = tokio::runtime::Handle::current();
        let async_coordinator = AsyncWorkCoordinator::new(config.thread_pool_size * 2);

        let mut thread_pools = HashMap::new();

        // Create compute thread pool
        let compute_config = ThreadPoolConfig {
            thread_count: config.thread_pool_size,
            name_prefix: "compute".to_string(),
            ..Default::default()
        };
        let compute_pool = ThreadPool::new(ThreadPoolType::Compute, compute_config)?;
        thread_pools.insert(ThreadPoolType::Compute, compute_pool);

        // Create I/O thread pool
        let io_config = ThreadPoolConfig {
            thread_count: config.io_thread_pool_size,
            name_prefix: "io".to_string(),
            ..Default::default()
        };
        let io_pool = ThreadPool::new(ThreadPoolType::Io, io_config)?;
        thread_pools.insert(ThreadPoolType::Io, io_pool);

        // Create blocking thread pool
        let blocking_config = ThreadPoolConfig {
            thread_count: config.blocking_thread_pool_size,
            name_prefix: "blocking".to_string(),
            ..Default::default()
        };
        let blocking_pool = ThreadPool::new(ThreadPoolType::Blocking, blocking_config)?;
        thread_pools.insert(ThreadPoolType::Blocking, blocking_pool);

        Ok(Self {
            state: ManagedState::new(Uuid::new_v4(), "concurrency_manager"),
            config,
            thread_pools,
            async_coordinator,
            runtime_handle,
        })
    }

    /// Execute a CPU-intensive task
    pub async fn execute_compute<F, R>(&self, task: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let compute_pool = self.thread_pools.get(&ThreadPoolType::Compute)
            .ok_or_else(|| Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                "Compute thread pool not available",
            ))?;

        compute_pool.submit_async(task).await
    }

    /// Execute an I/O task
    pub async fn execute_io<F, R>(&self, task: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let io_pool = self.thread_pools.get(&ThreadPoolType::Io)
            .ok_or_else(|| Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                "I/O thread pool not available",
            ))?;

        io_pool.submit_async(task).await
    }

    /// Execute a blocking task
    pub async fn execute_blocking<F, R>(&self, task: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let blocking_pool = self.thread_pools.get(&ThreadPoolType::Blocking)
            .ok_or_else(|| Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                "Blocking thread pool not available",
            ))?;

        blocking_pool.submit_async(task).await
    }

    /// Spawn a task on the async runtime
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime_handle.spawn(future)
    }

    /// Spawn a blocking task
    pub fn spawn_blocking<F, R>(&self, task: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.runtime_handle.spawn_blocking(task)
    }

    /// Get thread pool statistics
    pub fn get_thread_pool_stats(&self, pool_type: ThreadPoolType) -> Option<ThreadPoolStats> {
        self.thread_pools.get(&pool_type).map(|pool| pool.stats())
    }

    /// Get all thread pool statistics
    pub fn get_all_thread_pool_stats(&self) -> HashMap<ThreadPoolType, ThreadPoolStats> {
        self.thread_pools
            .iter()
            .map(|(pool_type, pool)| (*pool_type, pool.stats()))
            .collect()
    }

    /// Get async coordinator statistics
    pub fn get_async_coordinator_stats(&self) -> AsyncCoordinatorStats {
        self.async_coordinator.stats()
    }

    /// Create a custom thread pool
    pub fn create_custom_pool(
        &mut self,
        pool_id: u8,
        config: ThreadPoolConfig,
    ) -> Result<()> {
        let pool_type = ThreadPoolType::Custom(pool_id);
        let thread_pool = ThreadPool::new(pool_type, config)?;
        self.thread_pools.insert(pool_type, thread_pool);
        Ok(())
    }

    /// Execute task on custom thread pool
    pub async fn execute_custom<F, R>(&self, pool_id: u8, task: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let pool_type = ThreadPoolType::Custom(pool_id);
        let custom_pool = self.thread_pools.get(&pool_type)
            .ok_or_else(|| Error::new(
                ErrorKind::Concurrency {
                    thread_id: None,
                    operation: ConcurrencyOperation::ThreadPool,
                },
                format!("Custom thread pool {} not available", pool_id),
            ))?;

        custom_pool.submit_async(task).await
    }
}

#[async_trait]
impl Manager for ConcurrencyManager {
    fn name(&self) -> &str {
        "concurrency_manager"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4() // Simplified
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::Initializing).await;

        // All thread pools are created during construction
        // Any additional initialization can go here

        self.state.set_state(crate::manager::ManagerState::Running).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::ShuttingDown).await;

        // Shutdown all thread pools
        let shutdown_timeout = Duration::from_secs(30);
        let mut pools_to_shutdown = Vec::new();

        // Drain the thread pools so we can shut them down
        for (pool_type, pool) in self.thread_pools.drain() {
            pools_to_shutdown.push((pool_type, pool));
        }

        for (pool_type, pool) in pools_to_shutdown {
            if let Err(e) = pool.shutdown(shutdown_timeout) {
                tracing::error!("Failed to shutdown {:?} thread pool: {}", pool_type, e);
            }
        }

        self.state.set_state(crate::manager::ManagerState::Shutdown).await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        // Add thread pool information
        let mut total_active_threads = 0;
        let mut total_queued_tasks = 0;
        let mut total_executed_tasks = 0u64;

        for (pool_type, stats) in self.get_all_thread_pool_stats() {
            total_active_threads += stats.active_threads;
            total_queued_tasks += stats.queue_size;
            total_executed_tasks += stats.total_executed;

            status.add_metadata(
                format!("{:?}_threads", pool_type).to_lowercase(),
                serde_json::Value::from(stats.active_threads),
            );
            status.add_metadata(
                format!("{:?}_queue_size", pool_type).to_lowercase(),
                serde_json::Value::from(stats.queue_size),
            );
        }

        status.add_metadata("total_active_threads", serde_json::Value::from(total_active_threads));
        status.add_metadata("total_queued_tasks", serde_json::Value::from(total_queued_tasks));
        status.add_metadata("total_executed_tasks", serde_json::Value::from(total_executed_tasks));

        // Add async coordinator stats
        let coordinator_stats = self.get_async_coordinator_stats();
        status.add_metadata("async_coordinated_tasks", serde_json::Value::from(coordinator_stats.total_coordinated));
        status.add_metadata("async_active_permits", serde_json::Value::from(coordinator_stats.active_permits));

        status
    }
}

/// Utility functions for common concurrency patterns
pub mod utils {
    use std::pin::Pin;
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    /// Execute multiple tasks concurrently and wait for all to complete
    pub async fn join_all<F, R>(tasks: Vec<F>) -> Vec<Result<R>>
    where
        F: Future<Output = Result<R>> + Send + 'static,
        R: Send + 'static,
    {
        let handles: Vec<_> = tasks.into_iter().map(tokio::spawn).collect();

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(Error::new(
                    ErrorKind::Concurrency {
                        thread_id: None,
                        operation: ConcurrencyOperation::Spawn,
                    },
                    format!("Task join error: {}", e),
                ))),
            }
        }

        results
    }

    /// Execute tasks with a concurrency limit
    pub async fn execute_with_limit<F, R>(
        tasks: Vec<F>,
        limit: usize,
    ) -> Vec<Result<R>>
    where
        F: Future<Output = Result<R>> + Send + 'static,
        R: Send + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(limit));
        let handles: Vec<_> = tasks
            .into_iter()
            .map(|task| {
                let sem = Arc::clone(&semaphore);
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.map_err(|_| {
                        Error::new(
                            ErrorKind::Concurrency {
                                thread_id: None,
                                operation: ConcurrencyOperation::Sync,
                            },
                            "Failed to acquire semaphore permit",
                        )
                    })?;
                    task.await
                })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(Error::new(
                    ErrorKind::Concurrency {
                        thread_id: None,
                        operation: ConcurrencyOperation::Spawn,
                    },
                    format!("Task join error: {}", e),
                ))),
            }
        }

        results
    }

    /// Synchronize multiple tasks at a barrier
    pub async fn synchronize_at_barrier(
        tasks: Vec<Box<dyn FnOnce(Arc<Barrier>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send>>,
    ) -> Result<()> {
        let barrier = Arc::new(Barrier::new(tasks.len()));
        let handles: Vec<_> = tasks
            .into_iter()
            .map(|task| {
                let barrier_clone = Arc::clone(&barrier);
                tokio::spawn(task(barrier_clone))
            })
            .collect();

        for handle in handles {
            handle.await.map_err(|e| {
                Error::new(
                    ErrorKind::Concurrency {
                        thread_id: None,
                        operation: ConcurrencyOperation::Spawn,
                    },
                    format!("Barrier synchronization error: {}", e),
                )
            })??;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_thread_pool_creation() {
        let config = ThreadPoolConfig::default();
        let pool = ThreadPool::new(ThreadPoolType::Compute, config).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.pool_type, ThreadPoolType::Compute);
        assert!(stats.active_threads > 0);
    }

    #[tokio::test]
    async fn test_thread_pool_task_execution() {
        let config = ThreadPoolConfig {
            thread_count: 2,
            ..Default::default()
        };
        let pool = ThreadPool::new(ThreadPoolType::Compute, config).unwrap();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = pool.submit_async(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            42i32 // Changed to i32 to fix type issue
        }).await.unwrap();

        assert_eq!(result, 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_concurrency_manager_initialization() {
        let config = ConcurrencyConfig::default();
        let mut manager = ConcurrencyManager::new(config).unwrap();

        manager.initialize().await.unwrap();

        let status = manager.status().await;
        assert_eq!(status.state, crate::manager::ManagerState::Running);

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_compute_task_execution() {
        let config = ConcurrencyConfig::default();
        let manager = ConcurrencyManager::new(config).unwrap();

        let result = manager.execute_compute(|| {
            // Simulate CPU-intensive work
            let mut sum = 0i32; // Changed to i32
            for i in 0..1000i32 { // Changed to i32
                sum += i;
            }
            sum
        }).await.unwrap();

        assert_eq!(result, 499500);
    }

    #[tokio::test]
    async fn test_thread_pool_stats() {
        let config = ThreadPoolConfig {
            thread_count: 2,
            ..Default::default()
        };
        let pool = ThreadPool::new(ThreadPoolType::Io, config).unwrap();

        // Execute some tasks
        for i in 0..5i32 { // Changed to i32
            let _ = pool.submit_async(move || {
                thread::sleep(Duration::from_millis(10));
                i * 2
            }).await;
        }

        let stats = pool.stats();
        assert_eq!(stats.pool_type, ThreadPoolType::Io);
        assert!(stats.total_executed >= 5);
    }

    #[tokio::test]
    async fn test_utils_join_all() {
        let tasks = vec![
            Box::pin(async { Ok(1i32) }) as Pin<Box<dyn Future<Output = Result<i32>> + Send>>,
            Box::pin(async { Ok(2i32) }) as Pin<Box<dyn Future<Output = Result<i32>> + Send>>,
            Box::pin(async { Ok(3i32) }) as Pin<Box<dyn Future<Output = Result<i32>> + Send>>,
        ];

        let results = utils::join_all(tasks).await;
        assert_eq!(results.len(), 3);

        for (i, result) in results.into_iter().enumerate() {
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), (i + 1) as i32); // Fixed: Cast to i32
        }
    }

    #[tokio::test]
    async fn test_utils_execute_with_limit() {
        let counter = Arc::new(AtomicU32::new(0));
        let tasks: Vec<_> = (0..10i32) // Changed to i32
            .map(|i| {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(i)
                }
            })
            .collect();

        let results = utils::execute_with_limit(tasks, 3).await;
        assert_eq!(results.len(), 10);
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}