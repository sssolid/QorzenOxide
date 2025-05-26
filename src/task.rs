// src/task.rs

//! Async task management system with progress tracking and lifecycle management

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock, Semaphore};
use tokio::time::{timeout, Instant};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::event::{Event, EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus};
use crate::config::TaskConfig;
use crate::types::{CorrelationId, Metadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 50,
    High = 100,
    Critical = 200,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    Core,
    Plugin,
    Ui,
    Io,
    Background,
    User,
    Maintenance,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub percent: u8,
    pub message: String,
    pub current_step: Option<u32>,
    pub total_steps: Option<u32>,
    pub updated_at: DateTime<Utc>,
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
    pub fn new(percent: u8, message: impl Into<String>) -> Self {
        Self {
            percent: percent.min(100),
            message: message.into(),
            updated_at: Utc::now(),
            ..Default::default()
        }
    }

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

    pub fn set_percent(&mut self, percent: u8) {
        self.percent = percent.min(100);
        self.updated_at = Utc::now();
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.updated_at = Utc::now();
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration: Duration,
    pub resource_usage: ResourceUsage,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub peak_memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub file_operations: u32,
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

pub trait ProgressReporter: Send + Sync + fmt::Debug {
    fn report(&self, progress: TaskProgress);
    fn report_percent(&self, percent: u8, message: String) {
        self.report(TaskProgress::new(percent, message));
    }
    fn report_step(&self, current: u32, total: u32, message: String) {
        self.report(TaskProgress::with_steps(current, total, message));
    }
}

#[derive(Debug)]
pub struct TaskContext {
    pub task_id: Uuid,
    pub name: String,
    pub category: TaskCategory,
    pub plugin_id: Option<String>,
    pub correlation_id: Option<CorrelationId>,
    pub progress: Arc<dyn ProgressReporter>,
    pub cancellation_token: tokio_util::sync::CancellationToken,
    pub metadata: Metadata,
}

impl TaskContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    pub async fn cancelled(&self) {
        self.cancellation_token.cancelled().await;
    }

    pub fn report_progress(&self, progress: TaskProgress) {
        self.progress.report(progress);
    }

    pub fn report_percent(&self, percent: u8, message: impl Into<String>) {
        self.progress.report_percent(percent, message.into());
    }

    pub fn report_step(&self, current: u32, total: u32, message: impl Into<String>) {
        self.progress.report_step(current, total, message.into());
    }
}

pub type TaskFunction = Arc<
    dyn Fn(TaskContext) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

pub struct TaskDefinition {
    pub id: Uuid,
    pub name: String,
    pub category: TaskCategory,
    pub priority: TaskPriority,
    pub plugin_id: Option<String>,
    pub dependencies: Vec<Uuid>,
    pub timeout: Duration,
    pub max_retries: u32,
    pub cancellable: bool,
    pub metadata: Metadata,
    pub correlation_id: Option<CorrelationId>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: Uuid,
    pub name: String,
    pub category: TaskCategory,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub plugin_id: Option<String>,
    pub dependencies: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: TaskProgress,
    pub result: Option<TaskResult>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timeout: Duration,
    pub cancellable: bool,
    pub correlation_id: Option<CorrelationId>,
    pub metadata: Metadata,
}

impl TaskInfo {
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

    pub fn duration(&self) -> Option<Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            Some((completed - started).to_std().ok()?)
        } else {
            None
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::TimedOut
        )
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.status, TaskStatus::Failed | TaskStatus::TimedOut)
            && self.retry_count < self.max_retries
    }
}

#[derive(Debug)]
struct TaskExecution {
    info: TaskInfo,
    definition: TaskDefinition,
    cancellation_token: tokio_util::sync::CancellationToken,
    progress_sender: broadcast::Sender<TaskProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManagerStats {
    pub total_created: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub currently_running: u32,
    pub currently_pending: u32,
    pub by_category: HashMap<String, u64>,
    pub by_priority: HashMap<TaskPriority, u64>,
    pub avg_execution_time_ms: f64,
    pub total_resource_usage: ResourceUsage,
}

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

#[derive(Debug)]
struct TaskProgressReporter {
    task_id: Uuid,
    progress_sender: broadcast::Sender<TaskProgress>,
}

impl ProgressReporter for TaskProgressReporter {
    fn report(&self, progress: TaskProgress) {
        tracing::debug!(
            "Task {} progress: {}% - {}",
            self.task_id,
            progress.percent,
            progress.message
        );
        let _ = self.progress_sender.send(progress);
    }
}

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
    shutdown_flag: Arc<tokio::sync::RwLock<bool>>,
}

impl TaskManager {
    pub fn new(config: TaskConfig) -> Self {
        let max_concurrent = config.max_concurrent;
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
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            event_bus: None,
            worker_handles: Vec::new(),
            shutdown_flag: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    pub async fn submit_task(&self, definition: TaskDefinition) -> Result<Uuid> {
        let task_id = definition.id;
        let task_info = TaskInfo::from_definition(&definition);

        tracing::info!("Submitting task {} ({})", task_info.name, task_id);

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
        tracing::debug!(
            "Task {} added to collection, total tasks: {}",
            task_id,
            self.tasks.len()
        );

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_created += 1;
            stats.currently_pending += 1;
            *stats
                .by_category
                .entry(task_info.category.to_string())
                .or_insert(0) += 1;
            *stats.by_priority.entry(task_info.priority).or_insert(0) += 1;
            tracing::debug!("Stats updated: {} pending tasks", stats.currently_pending);
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

        tracing::info!("Task {} submitted successfully", task_id);
        Ok(task_id)
    }

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
                    stats.currently_running = stats.currently_running.saturating_sub(1);
                } else {
                    stats.currently_pending = stats.currently_pending.saturating_sub(1);
                }
            }

            // Publish status change event
            self.publish_status_change_event(
                &task.info,
                TaskStatus::Running,
                TaskStatus::Cancelled,
            )
            .await;

            Ok(true)
        } else {
            Err(Error::task(Some(task_id), "Task not found"))
        }
    }

    pub async fn get_task_info(&self, task_id: Uuid) -> Option<TaskInfo> {
        self.tasks.get(&task_id).map(|task| task.info.clone())
    }

    pub async fn list_tasks(
        &self,
        status_filter: Option<TaskStatus>,
        category_filter: Option<TaskCategory>,
        limit: Option<usize>,
    ) -> Vec<TaskInfo> {
        let tasks: Vec<TaskInfo> = self
            .tasks
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

    pub async fn wait_for_task(
        &self,
        task_id: Uuid,
        timeout_duration: Option<Duration>,
    ) -> Result<TaskInfo> {
        tracing::info!(
            "Waiting for task {} with timeout {:?}",
            task_id,
            timeout_duration
        );

        if let Some(task) = self.tasks.get(&task_id) {
            if task.info.is_terminal() {
                tracing::info!(
                    "Task {} already completed with status: {:?}",
                    task_id,
                    task.info.status
                );
                return Ok(task.info.clone());
            }

            let progress_receiver = task.progress_sender.subscribe();
            drop(task); // Release the DashMap reference

            let wait_future = self.wait_for_completion(task_id, progress_receiver);

            if let Some(timeout_duration) = timeout_duration {
                match timeout(timeout_duration, wait_future).await {
                    Ok(result) => result,
                    Err(_) => {
                        tracing::error!(
                            "Task {} wait timed out after {:?}",
                            task_id,
                            timeout_duration
                        );
                        Err(Error::timeout("Task wait timeout"))
                    }
                }
            } else {
                wait_future.await
            }
        } else {
            Err(Error::task(Some(task_id), "Task not found"))
        }
    }

    async fn wait_for_completion(
        &self,
        task_id: Uuid,
        mut progress_receiver: broadcast::Receiver<TaskProgress>,
    ) -> Result<TaskInfo> {
        loop {
            // Check if task is completed
            if let Some(updated_task) = self.tasks.get(&task_id) {
                if updated_task.info.is_terminal() {
                    tracing::info!(
                        "Task {} completed with status: {:?}",
                        task_id,
                        updated_task.info.status
                    );
                    return Ok(updated_task.info.clone());
                }
            }

            // Wait for progress update or timeout
            match tokio::time::timeout(Duration::from_millis(500), progress_receiver.recv()).await {
                Ok(Ok(progress)) => {
                    tracing::debug!(
                        "Task {} progress: {}% - {}",
                        task_id,
                        progress.percent,
                        progress.message
                    );
                    continue;
                }
                Ok(Err(_)) => {
                    // Channel closed, check final status
                    if let Some(updated_task) = self.tasks.get(&task_id) {
                        if updated_task.info.is_terminal() {
                            return Ok(updated_task.info.clone());
                        }
                    }
                    tracing::warn!("Progress channel closed for task {}", task_id);
                    break;
                }
                Err(_) => {
                    // Timeout on progress, check task status anyway
                    if let Some(updated_task) = self.tasks.get(&task_id) {
                        tracing::debug!(
                            "Task {} current status: {:?}",
                            task_id,
                            updated_task.info.status
                        );
                        if updated_task.info.is_terminal() {
                            return Ok(updated_task.info.clone());
                        }
                    }
                }
            }
        }

        Err(Error::task(Some(task_id), "Task completion wait failed"))
    }

    pub async fn get_stats(&self) -> TaskManagerStats {
        self.stats.read().await.clone()
    }

    pub async fn cleanup_old_tasks(&self, max_age: Duration) -> u64 {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        let mut removed_count = 0u64;

        let task_ids_to_remove: Vec<Uuid> = self
            .tasks
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

    async fn start_workers(&mut self) -> Result<()> {
        let worker_count = 4;
        tracing::info!("Starting {} task workers", worker_count);

        for worker_id in 0..worker_count {
            let tasks = Arc::clone(&self.tasks);
            let stats = Arc::clone(&self.stats);
            let semaphore = Arc::clone(&self.concurrency_semaphore);
            let event_bus = self.event_bus.clone();
            let shutdown_flag = Arc::clone(&self.shutdown_flag);

            let handle = tokio::spawn(async move {
                Self::task_worker(worker_id, tasks, stats, semaphore, event_bus, shutdown_flag)
                    .await;
            });

            self.worker_handles.push(handle);
        }

        tracing::info!("Started {} task workers", worker_count);
        Ok(())
    }

    async fn task_worker(
        worker_id: usize,
        tasks: Arc<DashMap<Uuid, TaskExecution>>,
        stats: Arc<RwLock<TaskManagerStats>>,
        semaphore: Arc<Semaphore>,
        event_bus: Option<Arc<EventBusManager>>,
        shutdown_flag: Arc<tokio::sync::RwLock<bool>>,
    ) {
        tracing::info!("Task worker {} started", worker_id);

        loop {
            // Check shutdown flag
            if *shutdown_flag.read().await {
                tracing::info!("Task worker {} shutting down", worker_id);
                break;
            }

            // Try to find and claim a pending task atomically
            let claimed_task_id = {
                let mut claimed = None;
                for mut entry in tasks.iter_mut() {
                    let task_id = *entry.key();
                    let task_ref = entry.value_mut(); // ✅ mutable access

                    if task_ref.info.status == TaskStatus::Pending {
                        task_ref.info.status = TaskStatus::Running;
                        task_ref.info.started_at = Some(Utc::now());
                        claimed = Some(task_id);
                        break;
                    }
                }
                claimed
            };

            if let Some(task_id) = claimed_task_id {
                tracing::info!("Worker {} claimed task {}", worker_id, task_id);

                // Acquire semaphore permit
                if let Ok(permit) = semaphore.acquire().await {
                    // Extract the task data we need for execution
                    let task_execution_data = {
                        if let Some(task_entry) = tasks.get(&task_id) {
                            let task = task_entry.value();

                            // Update statistics
                            {
                                let mut stats_guard = stats.write().await;
                                stats_guard.currently_pending =
                                    stats_guard.currently_pending.saturating_sub(1);
                                stats_guard.currently_running += 1;
                                tracing::debug!(
                                    "Stats: {} pending, {} running",
                                    stats_guard.currently_pending,
                                    stats_guard.currently_running
                                );
                            }

                            // Publish status change event
                            if let Some(event_bus) = &event_bus {
                                let event = TaskStatusChangedEvent {
                                    task_id,
                                    name: task.info.name.clone(),
                                    old_status: TaskStatus::Pending,
                                    new_status: TaskStatus::Running,
                                    timestamp: Utc::now(),
                                    source: "task_manager".to_string(),
                                    metadata: task.info.metadata.clone(),
                                };
                                let _ = event_bus.publish(event).await;
                            }

                            // Create context for task execution
                            let context = TaskContext {
                                task_id,
                                name: task.info.name.clone(),
                                category: task.info.category.clone(),
                                plugin_id: task.info.plugin_id.clone(),
                                correlation_id: task.info.correlation_id,
                                progress: Arc::new(TaskProgressReporter {
                                    task_id,
                                    progress_sender: task.progress_sender.clone(),
                                }),
                                cancellation_token: task.cancellation_token.clone(),
                                metadata: task.info.metadata.clone(),
                            };

                            Some((
                                Arc::clone(&task.definition.function),
                                context,
                                task.info.timeout,
                                task.progress_sender.clone(),
                            ))
                        } else {
                            None
                        }
                    };

                    if let Some((function, context, task_timeout, progress_sender)) =
                        task_execution_data
                    {
                        // Execute the task function
                        let start_time = Instant::now();
                        tracing::info!(
                            "Worker {} executing task {} with timeout {:?}",
                            worker_id,
                            task_id,
                            task_timeout
                        );

                        // Call the function to get the future
                        let future = function(context);

                        // Execute with timeout
                        let execution_result = timeout(task_timeout, future).await;
                        let execution_duration = start_time.elapsed();

                        tracing::info!(
                            "Task {} execution completed in {:?}",
                            task_id,
                            execution_duration
                        );

                        // Update task with result
                        let (new_status, result) = match execution_result {
                            Ok(Ok(data)) => {
                                tracing::info!("Task {} completed successfully", task_id);
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
                                tracing::error!("Task {} failed: {}", task_id, error);
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
                                tracing::error!("Task {} timed out", task_id);
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
                        if let Some(mut task_entry) = tasks.get_mut(&task_id) {
                            let task = task_entry.value_mut();
                            task.info.status = new_status;
                            task.info.completed_at = Some(Utc::now());
                            task.info.result = result;

                            // Send final progress update
                            let final_progress = TaskProgress::new(100, "Task completed");
                            let _ = progress_sender.send(final_progress);

                            tracing::info!("Task {} final status: {:?}", task_id, new_status);
                        }

                        // Update statistics
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.currently_running =
                                stats_guard.currently_running.saturating_sub(1);
                            match new_status {
                                TaskStatus::Completed => stats_guard.total_completed += 1,
                                TaskStatus::Failed => stats_guard.total_failed += 1,
                                TaskStatus::TimedOut => stats_guard.total_failed += 1,
                                _ => {}
                            }

                            // Update average execution time
                            let total_completed =
                                stats_guard.total_completed + stats_guard.total_failed;
                            if total_completed > 0 {
                                stats_guard.avg_execution_time_ms = (stats_guard
                                    .avg_execution_time_ms
                                    * (total_completed - 1) as f64
                                    + execution_duration.as_millis() as f64)
                                    / total_completed as f64;
                            }

                            tracing::debug!(
                                "Updated stats: {} completed, {} failed, {} running",
                                stats_guard.total_completed,
                                stats_guard.total_failed,
                                stats_guard.currently_running
                            );
                        }

                        // Publish status change event
                        if let Some(event_bus) = &event_bus {
                            let event = TaskStatusChangedEvent {
                                task_id,
                                name: tasks
                                    .get(&task_id)
                                    .map(|t| t.info.name.clone())
                                    .unwrap_or_default(),
                                old_status: TaskStatus::Running,
                                new_status,
                                timestamp: Utc::now(),
                                source: "task_manager".to_string(),
                                metadata: HashMap::new(),
                            };
                            let _ = event_bus.publish(event).await;
                        }
                    }

                    drop(permit); // Release semaphore
                }
            } else {
                // No pending tasks, wait a bit
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        tracing::info!("Task worker {} stopped", worker_id);
    }

    async fn publish_status_change_event(
        &self,
        task_info: &TaskInfo,
        old_status: TaskStatus,
        new_status: TaskStatus,
    ) {
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

    async fn stop_workers(&mut self) {
        tracing::info!("Stopping task workers");

        // Set shutdown flag
        *self.shutdown_flag.write().await = true;

        // Wait for workers to finish
        for handle in self.worker_handles.drain(..) {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        tracing::info!("All task workers stopped");
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
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        tracing::info!("Initializing task manager");

        // Start task execution workers
        self.start_workers().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        tracing::info!("Task manager initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        tracing::info!("Shutting down task manager");

        // Cancel all running tasks
        let running_tasks: Vec<Uuid> = self
            .tasks
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

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        tracing::info!("Task manager shutdown complete");
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_stats().await;

        status.add_metadata("total_tasks", serde_json::Value::from(stats.total_created));
        status.add_metadata(
            "completed_tasks",
            serde_json::Value::from(stats.total_completed),
        );
        status.add_metadata("failed_tasks", serde_json::Value::from(stats.total_failed));
        status.add_metadata(
            "running_tasks",
            serde_json::Value::from(stats.currently_running),
        );
        status.add_metadata(
            "pending_tasks",
            serde_json::Value::from(stats.currently_pending),
        );
        status.add_metadata(
            "avg_execution_time_ms",
            serde_json::Value::from(stats.avg_execution_time_ms),
        );

        status
    }
}

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

    pub fn category(mut self, category: TaskCategory) -> Self {
        self.category = category;
        self
    }

    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn plugin_id(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin_id = Some(plugin_id.into());
        self
    }

    pub fn dependency(mut self, task_id: Uuid) -> Self {
        self.dependencies.push(task_id);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn correlation_id(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn build<F, Fut>(self, function: F) -> TaskDefinition
    where
        F: Fn(TaskContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let task_function: TaskFunction = Arc::new(move |ctx| Box::pin(function(ctx)));

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
            .metadata(
                "key".to_string(),
                serde_json::Value::String("value".to_string()),
            )
            .build(|_ctx| async { Ok(serde_json::Value::String("completed".to_string())) });

        assert_eq!(task.name, "test_task");
        assert_eq!(task.category, TaskCategory::User);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.timeout, Duration::from_secs(60));
        assert!(task.cancellable);
        assert!(task.metadata.contains_key("key"));
    }

    #[tokio::test]
    async fn test_task_submission_and_execution() {
        let config = TaskConfig::default();
        let mut manager = TaskManager::new(config);
        manager.initialize().await.unwrap();

        let task = TaskBuilder::new("test_task")
            .timeout(Duration::from_secs(10))
            .build(|ctx| async move {
                ctx.report_percent(50, "Half way done");
                tokio::time::sleep(Duration::from_millis(100)).await;
                ctx.report_percent(100, "Complete");
                Ok(serde_json::Value::String("completed".to_string()))
            });

        let task_id = manager.submit_task(task).await.unwrap();

        let task_info = manager
            .wait_for_task(task_id, Some(Duration::from_secs(5)))
            .await
            .unwrap();
        assert_eq!(task_info.name, "test_task");
        assert_eq!(task_info.status, TaskStatus::Completed);

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
        assert_eq!(
            TaskCategory::Custom("custom".to_string()).to_string(),
            "custom"
        );
    }
}
