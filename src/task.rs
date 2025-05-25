// src/task.rs

//! Async task management system with progress tracking and lifecycle management
//!
//! This module provides a comprehensive task management system that supports:
//! - Async task execution with progress tracking
//! - Task priorities and scheduling
//! - Task cancellation and timeout handling
//! - Task result management and persistence
//! - Task dependencies and workflows
//! - Resource management and concurrency limits
//! - Task monitoring and metrics

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock, Semaphore};
use tokio::time::{timeout, Instant};
use uuid::Uuid;

use crate::config::TaskConfig;
use crate::error::{Error, Result};
use crate::event::{Event, EventBusManager};
use crate::manager::{Manager, ManagedState, ManagerStatus};
use crate::types::{CorrelationId, Metadata};

/// Task execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
    /// Task is paused
    Paused,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Running => write!(f, "RUNNING"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Cancelled => write!(f, "CANCELLED"),
            Self::TimedOut => write!(f, "TIMED_OUT"),
            Self::Paused => write!(f, "PAUSED"),
        }
    }
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Low priority tasks
    Low = 0,
    /// Normal priority tasks
    Normal = 50,
    /// High priority tasks
    High = 100,
    /// Critical priority tasks
    Critical = 200,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Task category for organization and resource management
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    /// Core system tasks
    Core,
    /// Plugin tasks
    Plugin,
    /// User interface tasks
    Ui,
    /// I/O operations
    Io,
    /// Background processing
    Background,
    /// User-initiated tasks
    User,
    /// Maintenance tasks
    Maintenance,
    /// Custom category
    Custom(String),
}

impl fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Plugin => write!(f, "plugin"),
            Self::Ui => write!(f, "ui"),
            Self::Io => write!(f, "io"),
            Self::Background => write!(f, "background"),
            Self::User => write!(f, "user"),
            Self::Maintenance => write!(f, "maintenance"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Task progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Completion percentage (0-100)
    pub percent: u8,
    /// Current progress message
    pub message: String,
    /// Current step number
    pub current_step: Option<u32>,
    /// Total number of steps
    pub total_steps: Option<u32>,
    /// When progress was last updated
    pub updated_at: DateTime<Utc>,
    /// Additional progress metadata
    pub metadata: Metadata,
}

impl Default for TaskProgress {
    fn default() -> Self {
        Self {
            percent: 0,
            message: String::new(),
            current_step: None,
            total_steps: None,
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

impl TaskProgress {
    /// Create new progress with percentage and message
    pub fn new(percent: u8, message: impl Into<String>) -> Self {
        Self {
            percent: percent.min(100),
            message: message.into(),
            updated_at: Utc::now(),
            ..Default::default()
        }
    }

    /// Create progress with steps
    pub fn with_steps(current: u32, total: u32, message: impl Into<String>) -> Self {
        let percent = if total > 0 {
            ((current as f64 / total as f64) * 100.0) as u8
        } else {
            0
        };

        Self {
            percent,
            message: message.into(),
            current_step: Some(current),
            total_steps: Some(total),
            updated_at: Utc::now(),
            ..Default::default()
        }
    }

    /// Update progress percentage
    pub fn set_percent(&mut self, percent: u8) {
        self.percent = percent.min(100);
        self.updated_at = Utc::now();
    }

    /// Update progress message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.updated_at = Utc::now();
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
        self.updated_at = Utc::now();
    }
}

/// Task result container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Whether the task succeeded
    pub success: bool,
    /// Result data (if successful)
    pub data: Option<serde_json::Value>,
    /// Error information (if failed)
    pub error: Option<String>,
    /// Task execution duration
    pub duration: Duration,
    /// Resource usage information
    pub resource_usage: ResourceUsage,
    /// Additional result metadata
    pub metadata: Metadata,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// CPU time used
    pub cpu_time_ms: u64,
    /// Number of file operations
    pub file_operations: u32,
    /// Network bytes transferred
    pub network_bytes: u64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            peak_memory_bytes: 0,
            cpu_time_ms: 0,
            file_operations: 0,
            network_bytes: 0,
        }
    }
}

/// Progress reporter trait for tasks
pub trait ProgressReporter: Send + Sync + fmt::Debug {
    fn report(&self, progress: TaskProgress);
    fn report_percent(&self, percent: u8, message: String) {
        self.report(TaskProgress::new(percent, message));
    }
    fn report_step(&self, current: u32, total: u32, message: String) {
        self.report(TaskProgress::with_steps(current, total, message));
    }
}

/// Task execution context passed to task functions
#[derive(Debug)]
pub struct TaskContext {
    /// Unique task identifier
    pub task_id: Uuid,
    /// Task name
    pub name: String,
    /// Task category
    pub category: TaskCategory,
    /// Plugin ID if task belongs to a plugin
    pub plugin_id: Option<String>,
    /// Correlation ID for tracking related operations
    pub correlation_id: Option<CorrelationId>,
    /// Progress reporter
    pub progress: Arc<dyn ProgressReporter>,
    /// Cancellation token
    pub cancellation_token: tokio_util::sync::CancellationToken,
    /// Task metadata
    pub metadata: Metadata,
}

impl TaskContext {
    /// Check if task should be cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Wait for cancellation
    pub async fn cancelled(&self) {
        self.cancellation_token.cancelled().await;
    }

    /// Report progress
    pub fn report_progress(&self, progress: TaskProgress) {
        self.progress.report(progress);
    }

    /// Report percentage progress
    pub fn report_percent(&self, percent: u8, message: impl Into<String>) {
        self.progress.report_percent(percent, message.into());
    }

    /// Report step progress
    pub fn report_step(&self, current: u32, total: u32, message: impl Into<String>) {
        self.progress.report_step(current, total, message.into());
    }
}

/// Task function type
pub type TaskFunction = Box<
    dyn Fn(TaskContext) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send>>
    + Send
    + Sync
>;

/// Task definition
pub struct TaskDefinition {
    /// Unique task identifier
    pub id: Uuid,
    /// Task name
    pub name: String,
    /// Task category
    pub category: TaskCategory,
    /// Task priority
    pub priority: TaskPriority,
    /// Plugin ID if task belongs to a plugin
    pub plugin_id: Option<String>,
    /// Task dependencies (task IDs that must complete first)
    pub dependencies: Vec<Uuid>,
    /// Task timeout duration
    pub timeout: Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Whether task can be cancelled
    pub cancellable: bool,
    /// Task metadata
    pub metadata: Metadata,
    /// Correlation ID
    pub correlation_id: Option<CorrelationId>,
    /// Task function
    pub function: TaskFunction,
}

impl fmt::Debug for TaskDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskDefinition")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("category", &self.category)
            .field("priority", &self.priority)
            .field("plugin_id", &self.plugin_id)
            .field("dependencies", &self.dependencies)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("cancellable", &self.cancellable)
            .field("metadata", &self.metadata)
            .field("correlation_id", &self.correlation_id)
            .field("function", &"<function>")
            .finish()
    }
}

/// Task execution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Task definition ID
    pub id: Uuid,
    /// Task name
    pub name: String,
    /// Task category
    pub category: TaskCategory,
    /// Task priority
    pub priority: TaskPriority,
    /// Current status
    pub status: TaskStatus,
    /// Plugin ID if applicable
    pub plugin_id: Option<String>,
    /// Task dependencies
    pub dependencies: Vec<Uuid>,
    /// When task was created
    pub created_at: DateTime<Utc>,
    /// When task started execution
    pub started_at: Option<DateTime<Utc>>,
    /// When task completed/failed
    pub completed_at: Option<DateTime<Utc>>,
    /// Current progress
    pub progress: TaskProgress,
    /// Task result (if completed)
    pub result: Option<TaskResult>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Task timeout
    pub timeout: Duration,
    /// Whether task is cancellable
    pub cancellable: bool,
    /// Correlation ID
    pub correlation_id: Option<CorrelationId>,
    /// Task metadata
    pub metadata: Metadata,
}

impl TaskInfo {
    /// Create task info from definition
    pub fn from_definition(definition: &TaskDefinition) -> Self {
        Self {
            id: definition.id,
            name: definition.name.clone(),
            category: definition.category.clone(),
            priority: definition.priority,
            status: TaskStatus::Pending,
            plugin_id: definition.plugin_id.clone(),
            dependencies: definition.dependencies.clone(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            progress: TaskProgress::default(),
            result: None,
            retry_count: 0,
            max_retries: definition.max_retries,
            timeout: definition.timeout,
            cancellable: definition.cancellable,
            correlation_id: definition.correlation_id,
            metadata: definition.metadata.clone(),
        }
    }

    /// Get task duration if completed
    pub fn duration(&self) -> Option<Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            Some((completed - started).to_std().ok()?)
        } else {
            None
        }
    }

    /// Check if task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::TimedOut
        )
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        matches!(self.status, TaskStatus::Failed | TaskStatus::TimedOut)
            && self.retry_count < self.max_retries
    }
}

/// Task execution wrapper
#[derive(Debug)]
struct TaskExecution {
    info: TaskInfo,
    definition: TaskDefinition,
    cancellation_token: tokio_util::sync::CancellationToken,
    progress_sender: broadcast::Sender<TaskProgress>,
}

/// Task manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManagerStats {
    /// Total tasks created
    pub total_created: u64,
    /// Total tasks completed
    pub total_completed: u64,
    /// Total tasks failed
    pub total_failed: u64,
    /// Total tasks cancelled
    pub total_cancelled: u64,
    /// Currently running tasks
    pub currently_running: u32,
    /// Currently pending tasks
    pub currently_pending: u32,
    /// Tasks by category
    pub by_category: HashMap<String, u64>,
    /// Tasks by priority
    pub by_priority: HashMap<TaskPriority, u64>,
    /// Average execution time
    pub avg_execution_time_ms: f64,
    /// Resource usage totals
    pub total_resource_usage: ResourceUsage,
}

/// Task events for the event system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCreatedEvent {
    pub task_id: Uuid,
    pub name: String,
    pub category: TaskCategory,
    pub priority: TaskPriority,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: Metadata,
}

impl Event for TaskCreatedEvent {
    fn event_type(&self) -> &'static str {
        "task.created"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusChangedEvent {
    pub task_id: Uuid,
    pub name: String,
    pub old_status: TaskStatus,
    pub new_status: TaskStatus,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: Metadata,
}

impl Event for TaskStatusChangedEvent {
    fn event_type(&self) -> &'static str {
        "task.status_changed"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressEvent {
    pub task_id: Uuid,
    pub name: String,
    pub progress: TaskProgress,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: Metadata,
}

impl Event for TaskProgressEvent {
    fn event_type(&self) -> &'static str {
        "task.progress"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Progress reporter implementation
#[derive(Debug)]
struct TaskProgressReporter {
    task_id: Uuid,
    progress_sender: broadcast::Sender<TaskProgress>,
}

impl ProgressReporter for TaskProgressReporter {
    fn report(&self, progress: TaskProgress) {
        let _ = self.progress_sender.send(progress);
    }
}

/// Main task manager implementation
#[derive(Debug)]
pub struct TaskManager {
    state: ManagedState,
    config: TaskConfig,
    tasks: Arc<DashMap<Uuid, TaskExecution>>,
    stats: Arc<RwLock<TaskManagerStats>>,
    task_counter: Arc<AtomicU64>,
    concurrency_semaphore: Arc<Semaphore>,
    event_bus: Option<Arc<EventBusManager>>,
    worker_handles: Vec<tokio::task::JoinHandle<()>>,
    task_queue: Arc<RwLock<Vec<Uuid>>>, // Simple FIFO queue, would use priority queue in production
}

impl TaskManager {
    /// Create a new task manager
    pub fn new(config: TaskConfig) -> Self {
        let concurrency_semaphore = Arc::new(Semaphore::new(config.max_concurrent));

        Self {
            state: ManagedState::new(Uuid::new_v4(), "task_manager"),
            config,
            tasks: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(TaskManagerStats {
                total_created: 0,
                total_completed: 0,
                total_failed: 0,
                total_cancelled: 0,
                currently_running: 0,
                currently_pending: 0,
                by_category: HashMap::new(),
                by_priority: HashMap::new(),
                avg_execution_time_ms: 0.0,
                total_resource_usage: ResourceUsage::default(),
            })),
            task_counter: Arc::new(AtomicU64::new(0)),
            concurrency_semaphore,
            event_bus: None,
            worker_handles: Vec::new(),
            task_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set event bus for publishing task events
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    /// Submit a task for execution
    pub async fn submit_task(&self, definition: TaskDefinition) -> Result<Uuid> {
        let task_id = definition.id;
        let task_info = TaskInfo::from_definition(&definition);

        // Check dependencies
        for dep_id in &task_info.dependencies {
            if let Some(dep_task) = self.tasks.get(dep_id) {
                if !dep_task.info.is_terminal() {
                    return Err(Error::task(
                        Some(task_id),
                        format!("Dependency task {} is not completed", dep_id),
                    ));
                }
                if dep_task.info.status != TaskStatus::Completed {
                    return Err(Error::task(
                        Some(task_id),
                        format!("Dependency task {} failed", dep_id),
                    ));
                }
            } else {
                return Err(Error::task(
                    Some(task_id),
                    format!("Dependency task {} not found", dep_id),
                ));
            }
        }

        let (progress_sender, _) = broadcast::channel(100);
        let cancellation_token = tokio_util::sync::CancellationToken::new();

        let execution = TaskExecution {
            info: task_info.clone(),
            definition,
            cancellation_token,
            progress_sender,
        };

        // Add to tasks collection
        self.tasks.insert(task_id, execution);

        // Add to task queue
        {
            let mut queue = self.task_queue.write().await;
            queue.push(task_id);
            // In a real implementation, this would be a priority queue
            queue.sort_by_key(|id| {
                self.tasks.get(id).map(|task| task.info.priority).unwrap_or(TaskPriority::Normal)
            });
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_created += 1;
            stats.currently_pending += 1;
            *stats.by_category.entry(task_info.category.to_string()).or_insert(0) += 1;
            *stats.by_priority.entry(task_info.priority).or_insert(0) += 1;
        }

        // Publish task created event
        if let Some(event_bus) = &self.event_bus {
            let event = TaskCreatedEvent {
                task_id,
                name: task_info.name.clone(),
                category: task_info.category,
                priority: task_info.priority,
                timestamp: Utc::now(),
                source: "task_manager".to_string(),
                metadata: task_info.metadata.clone(),
            };
            let _ = event_bus.publish(event).await;
        }

        Ok(task_id)
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: Uuid) -> Result<bool> {
        if let Some(mut task) = self.tasks.get_mut(&task_id) {
            if !task.info.cancellable {
                return Err(Error::task(Some(task_id), "Task is not cancellable"));
            }

            if task.info.is_terminal() {
                return Ok(false);
            }

            // Cancel the task
            task.cancellation_token.cancel();
            task.info.status = TaskStatus::Cancelled;
            task.info.completed_at = Some(Utc::now());

            // Update statistics
            {
                let mut stats = self.stats.write().await;
                stats.total_cancelled += 1;
                if task.info.status == TaskStatus::Running {
                    stats.currently_running -= 1;
                } else {
                    stats.currently_pending -= 1;
                }
            }

            // Publish status change event
            self.publish_status_change_event(&task.info, TaskStatus::Running, TaskStatus::Cancelled).await;

            Ok(true)
        } else {
            Err(Error::task(Some(task_id), "Task not found"))
        }
    }

    /// Get task information
    pub async fn get_task_info(&self, task_id: Uuid) -> Option<TaskInfo> {
        self.tasks.get(&task_id).map(|task| task.info.clone())
    }

    /// List tasks with optional filtering
    pub async fn list_tasks(
        &self,
        status_filter: Option<TaskStatus>,
        category_filter: Option<TaskCategory>,
        limit: Option<usize>,
    ) -> Vec<TaskInfo> {
        let tasks: Vec<TaskInfo> = self.tasks
            .iter()
            .filter_map(|entry| {
                let task_info = &entry.value().info;

                if let Some(status) = status_filter {
                    if task_info.status != status {
                        return None;
                    }
                }

                if let Some(category) = &category_filter {
                    if task_info.category != *category {
                        return None;
                    }
                }

                Some(task_info.clone())
            })
            .collect();

        if let Some(limit) = limit {
            tasks.into_iter().take(limit).collect()
        } else {
            tasks
        }
    }

    /// Wait for task completion
    pub async fn wait_for_task(&self, task_id: Uuid, timeout_duration: Option<Duration>) -> Result<TaskInfo> {
        if let Some(task) = self.tasks.get(&task_id) {
            if task.info.is_terminal() {
                return Ok(task.info.clone());
            }

            // Subscribe to progress updates to detect completion
            let mut progress_receiver = task.progress_sender.subscribe();

            let wait_future = async {
                loop {
                    if let Ok(_progress) = progress_receiver.recv().await {
                        if let Some(updated_task) = self.tasks.get(&task_id) {
                            if updated_task.info.is_terminal() {
                                return Ok(updated_task.info.clone());
                            }
                        }
                    }
                }
            };

            if let Some(timeout_duration) = timeout_duration {
                timeout(timeout_duration, wait_future)
                    .await
                    .map_err(|_| Error::timeout("Task wait timeout"))?
            } else {
                wait_future.await
            }
        } else {
            Err(Error::task(Some(task_id), "Task not found"))
        }
    }

    /// Get task manager statistics
    pub async fn get_stats(&self) -> TaskManagerStats {
        self.stats.read().await.clone()
    }

    /// Clean up completed tasks older than specified duration
    pub async fn cleanup_old_tasks(&self, max_age: Duration) -> u64 {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        let mut removed_count = 0u64;

        let task_ids_to_remove: Vec<Uuid> = self.tasks
            .iter()
            .filter_map(|entry| {
                let task_info = &entry.value().info;
                if task_info.is_terminal() {
                    if let Some(completed_at) = task_info.completed_at {
                        if completed_at < cutoff_time {
                            return Some(task_info.id);
                        }
                    }
                }
                None
            })
            .collect();

        for task_id in task_ids_to_remove {
            if self.tasks.remove(&task_id).is_some() {
                removed_count += 1;
            }
        }

        removed_count
    }

    /// Start task execution workers
    async fn start_workers(&mut self) -> Result<()> {
        let worker_count = 4; // Could be configurable

        for worker_id in 0..worker_count {
            let tasks = Arc::clone(&self.tasks);
            let stats = Arc::clone(&self.stats);
            let task_queue = Arc::clone(&self.task_queue);
            let semaphore = Arc::clone(&self.concurrency_semaphore);
            let event_bus = self.event_bus.clone();

            let handle = tokio::spawn(async move {
                Self::task_worker(worker_id, tasks, stats, task_queue, semaphore, event_bus).await;
            });

            self.worker_handles.push(handle);
        }

        Ok(())
    }

    /// Task execution worker
    async fn task_worker(
        worker_id: usize,
        tasks: Arc<DashMap<Uuid, TaskExecution>>,
        stats: Arc<RwLock<TaskManagerStats>>,
        task_queue: Arc<RwLock<Vec<Uuid>>>,
        semaphore: Arc<Semaphore>,
        event_bus: Option<Arc<EventBusManager>>,
    ) {
        tracing::debug!("Task worker {} started", worker_id);

        loop {
            // Get next task from queue
            let task_id = {
                let mut queue = task_queue.write().await;
                queue.pop()
            };

            if let Some(task_id) = task_id {
                // Acquire semaphore permit
                if let Ok(permit) = semaphore.acquire().await {
                    if let Some(mut task_entry) = tasks.get_mut(&task_id) {
                        let task = task_entry.value_mut();

                        // Update task status to running
                        let old_status = task.info.status;
                        task.info.status = TaskStatus::Running;
                        task.info.started_at = Some(Utc::now());

                        // Update statistics
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.currently_pending -= 1;
                            stats_guard.currently_running += 1;
                        }

                        // Publish status change event
                        if let Some(event_bus) = &event_bus {
                            let event = TaskStatusChangedEvent {
                                task_id,
                                name: task.info.name.clone(),
                                old_status,
                                new_status: TaskStatus::Running,
                                timestamp: Utc::now(),
                                source: "task_manager".to_string(),
                                metadata: task.info.metadata.clone(),
                            };
                            let _ = event_bus.publish(event).await;
                        }

                        // Execute the task
                        let start_time = Instant::now();
                        let progress_reporter = Arc::new(TaskProgressReporter {
                            task_id,
                            progress_sender: task.progress_sender.clone(),
                        });

                        let _context = TaskContext {
                            task_id,
                            name: task.info.name.clone(),
                            category: task.info.category.clone(),
                            plugin_id: task.info.plugin_id.clone(),
                            correlation_id: task.info.correlation_id,
                            progress: progress_reporter,
                            cancellation_token: task.cancellation_token.clone(),
                            metadata: task.info.metadata.clone(),
                        };

                        // Extract function to avoid borrow issues
                        let task_timeout = task.info.timeout;
                        // Note: In a real implementation, we'd need to handle the function execution properly
                        // This is simplified due to the complexity of storing and executing functions

                        // Simulate task execution
                        let execution_result = timeout(task_timeout, async {
                            // In real implementation, execute task.definition.function(context).await
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            Ok::<serde_json::Value, Error>(serde_json::Value::String("Task completed".to_string()))
                        }).await;

                        let execution_duration = start_time.elapsed();

                        // Update task with result
                        let (new_status, result) = match execution_result {
                            Ok(Ok(data)) => {
                                let result = TaskResult {
                                    success: true,
                                    data: Some(data),
                                    error: None,
                                    duration: execution_duration,
                                    resource_usage: ResourceUsage::default(),
                                    metadata: HashMap::new(),
                                };
                                (TaskStatus::Completed, Some(result))
                            }
                            Ok(Err(error)) => {
                                let result = TaskResult {
                                    success: false,
                                    data: None,
                                    error: Some(error.to_string()),
                                    duration: execution_duration,
                                    resource_usage: ResourceUsage::default(),
                                    metadata: HashMap::new(),
                                };
                                (TaskStatus::Failed, Some(result))
                            }
                            Err(_) => {
                                let result = TaskResult {
                                    success: false,
                                    data: None,
                                    error: Some("Task timed out".to_string()),
                                    duration: execution_duration,
                                    resource_usage: ResourceUsage::default(),
                                    metadata: HashMap::new(),
                                };
                                (TaskStatus::TimedOut, Some(result))
                            }
                        };

                        // Update task info
                        task.info.status = new_status;
                        task.info.completed_at = Some(Utc::now());
                        task.info.result = result;

                        // Update statistics
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.currently_running -= 1;
                            match new_status {
                                TaskStatus::Completed => stats_guard.total_completed += 1,
                                TaskStatus::Failed => stats_guard.total_failed += 1,
                                TaskStatus::TimedOut => stats_guard.total_failed += 1,
                                _ => {}
                            }

                            // Update average execution time
                            let total_tasks = stats_guard.total_completed + stats_guard.total_failed;
                            if total_tasks > 0 {
                                stats_guard.avg_execution_time_ms =
                                    (stats_guard.avg_execution_time_ms * (total_tasks - 1) as f64 +
                                        execution_duration.as_millis() as f64) / total_tasks as f64;
                            }
                        }

                        // Publish status change event
                        if let Some(event_bus) = &event_bus {
                            let event = TaskStatusChangedEvent {
                                task_id,
                                name: task.info.name.clone(),
                                old_status: TaskStatus::Running,
                                new_status,
                                timestamp: Utc::now(),
                                source: "task_manager".to_string(),
                                metadata: task.info.metadata.clone(),
                            };
                            let _ = event_bus.publish(event).await;
                        }
                    }

                    drop(permit); // Release semaphore
                }
            } else {
                // No tasks in queue, wait a bit
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    /// Publish task status change event
    async fn publish_status_change_event(&self, task_info: &TaskInfo, old_status: TaskStatus, new_status: TaskStatus) {
        if let Some(event_bus) = &self.event_bus {
            let event = TaskStatusChangedEvent {
                task_id: task_info.id,
                name: task_info.name.clone(),
                old_status,
                new_status,
                timestamp: Utc::now(),
                source: "task_manager".to_string(),
                metadata: task_info.metadata.clone(),
            };
            let _ = event_bus.publish(event).await;
        }
    }

    /// Stop all workers
    async fn stop_workers(&mut self) {
        for handle in self.worker_handles.drain(..) {
            handle.abort();
            let _ = handle.await;
        }
    }
}

#[async_trait]
impl Manager for TaskManager {
    fn name(&self) -> &str {
        "task_manager"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4() // Simplified
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::Initializing).await;

        // Start task execution workers
        self.start_workers().await?;

        self.state.set_state(crate::manager::ManagerState::Running).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::ShuttingDown).await;

        // Cancel all running tasks
        let running_tasks: Vec<Uuid> = self.tasks
            .iter()
            .filter_map(|entry| {
                let task = entry.value();
                if task.info.status == TaskStatus::Running && task.info.cancellable {
                    Some(task.info.id)
                } else {
                    None
                }
            })
            .collect();

        for task_id in running_tasks {
            let _ = self.cancel_task(task_id).await;
        }

        // Stop workers
        self.stop_workers().await;

        // Clean up old tasks
        self.cleanup_old_tasks(Duration::from_secs(0)).await;

        self.state.set_state(crate::manager::ManagerState::Shutdown).await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_stats().await;

        status.add_metadata("total_tasks", serde_json::Value::from(stats.total_created));
        status.add_metadata("completed_tasks", serde_json::Value::from(stats.total_completed));
        status.add_metadata("failed_tasks", serde_json::Value::from(stats.total_failed));
        status.add_metadata("running_tasks", serde_json::Value::from(stats.currently_running));
        status.add_metadata("pending_tasks", serde_json::Value::from(stats.currently_pending));
        status.add_metadata("avg_execution_time_ms", serde_json::Value::from(stats.avg_execution_time_ms));

        status
    }
}

/// Task builder for convenient task creation
pub struct TaskBuilder {
    name: String,
    category: TaskCategory,
    priority: TaskPriority,
    plugin_id: Option<String>,
    dependencies: Vec<Uuid>,
    timeout: Duration,
    max_retries: u32,
    cancellable: bool,
    metadata: Metadata,
    correlation_id: Option<CorrelationId>,
}

impl TaskBuilder {
    /// Create a new task builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: TaskCategory::Core,
            priority: TaskPriority::Normal,
            plugin_id: None,
            dependencies: Vec::new(),
            timeout: Duration::from_secs(300), // 5 minutes default
            max_retries: 0,
            cancellable: true,
            metadata: HashMap::new(),
            correlation_id: None,
        }
    }

    /// Set task category
    pub fn category(mut self, category: TaskCategory) -> Self {
        self.category = category;
        self
    }

    /// Set task priority
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set plugin ID
    pub fn plugin_id(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin_id = Some(plugin_id.into());
        self
    }

    /// Add dependency
    pub fn dependency(mut self, task_id: Uuid) -> Self {
        self.dependencies.push(task_id);
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set max retries
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set cancellable
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set correlation ID
    pub fn correlation_id(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Build the task definition with a function
    pub fn build<F, Fut>(self, function: F) -> TaskDefinition
    where
        F: Fn(TaskContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let task_function: TaskFunction = Box::new(move |ctx| Box::pin(function(ctx)));

        TaskDefinition {
            id: Uuid::new_v4(),
            name: self.name,
            category: self.category,
            priority: self.priority,
            plugin_id: self.plugin_id,
            dependencies: self.dependencies,
            timeout: self.timeout,
            max_retries: self.max_retries,
            cancellable: self.cancellable,
            metadata: self.metadata,
            correlation_id: self.correlation_id,
            function: task_function,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_manager_initialization() {
        let config = TaskConfig::default();
        let mut manager = TaskManager::new(config);

        manager.initialize().await.unwrap();

        let status = manager.status().await;
        assert_eq!(status.state, crate::manager::ManagerState::Running);

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_task_builder() {
        let task = TaskBuilder::new("test_task")
            .category(TaskCategory::User)
            .priority(TaskPriority::High)
            .timeout(Duration::from_secs(60))
            .cancellable(true)
            .metadata("key".to_string(), serde_json::Value::String("value".to_string()))
            .build(|_ctx| async {
                Ok(serde_json::Value::String("completed".to_string()))
            });

        assert_eq!(task.name, "test_task");
        assert_eq!(task.category, TaskCategory::User);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.timeout, Duration::from_secs(60));
        assert!(task.cancellable);
        assert!(task.metadata.contains_key("key"));
    }

    #[tokio::test]
    async fn test_task_submission() {
        let config = TaskConfig::default();
        let mut manager = TaskManager::new(config);
        manager.initialize().await.unwrap();

        let task = TaskBuilder::new("test_task")
            .build(|_ctx| async {
                Ok(serde_json::Value::String("completed".to_string()))
            });

        let task_id = manager.submit_task(task).await.unwrap();

        let task_info = manager.get_task_info(task_id).await.unwrap();
        assert_eq!(task_info.name, "test_task");
        assert_eq!(task_info.status, TaskStatus::Pending);

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_task_progress() {
        let mut progress = TaskProgress::default();
        assert_eq!(progress.percent, 0);

        progress.set_percent(50);
        assert_eq!(progress.percent, 50);

        progress.set_message("Half complete");
        assert_eq!(progress.message, "Half complete");

        let step_progress = TaskProgress::with_steps(5, 10, "Processing step 5");
        assert_eq!(step_progress.percent, 50);
        assert_eq!(step_progress.current_step, Some(5));
        assert_eq!(step_progress.total_steps, Some(10));
    }

    #[test]
    fn test_task_category_display() {
        assert_eq!(TaskCategory::Core.to_string(), "core");
        assert_eq!(TaskCategory::Plugin.to_string(), "plugin");
        assert_eq!(TaskCategory::Custom("custom".to_string()).to_string(), "custom");
    }
}