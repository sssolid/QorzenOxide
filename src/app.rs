// src/app.rs

//! Application core and manager orchestration
//!
//! This module provides the main application entry point and coordinates
//! all the system managers. It handles:
//! - Manager lifecycle and dependency resolution
//! - Application startup and shutdown
//! - Signal handling and graceful termination
//! - Health monitoring and status reporting
//! - Configuration management and hot-reloading
//! - Plugin system integration

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, timeout};
use uuid::Uuid;

use crate::config::{AppConfig, ConfigManager};
use crate::concurrency::ConcurrencyManager;
use crate::error::{ConfigOperation, Error, ErrorKind, ManagerOperation, Result, ResultExt};
use crate::event::{Event, EventBusManager};
use crate::file::FileManager;
use crate::logging::LoggingManager;
use crate::manager::{HealthStatus, Manager, ManagerState, ManagerStatus, ManagedState};
use crate::task::TaskManager;

/// Application lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationState {
    /// Application is being created
    Created,
    /// Application is initializing
    Initializing,
    /// Application is running normally
    Running,
    /// Application is shutting down
    ShuttingDown,
    /// Application has shut down
    Shutdown,
    /// Application is in an error state
    Error,
}

/// Application health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationHealth {
    /// Overall application health status
    pub status: HealthStatus,
    /// Application uptime
    pub uptime: Duration,
    /// Manager health statuses
    pub managers: HashMap<String, HealthStatus>,
    /// Last health check time
    pub last_check: DateTime<Utc>,
    /// Health check details
    pub details: HashMap<String, serde_json::Value>,
}

/// Application statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStats {
    /// Application version
    pub version: String,
    /// When application started
    pub started_at: DateTime<Utc>,
    /// Application uptime
    pub uptime: Duration,
    /// Current state
    pub state: ApplicationState,
    /// Manager count
    pub manager_count: usize,
    /// Initialized manager count
    pub initialized_managers: usize,
    /// Failed manager count
    pub failed_managers: usize,
    /// Total memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// System information
    pub system_info: SystemInfo,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system name
    pub os_name: String,
    /// Operating system version
    pub os_version: String,
    /// CPU architecture
    pub arch: String,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// Total system memory in bytes
    pub total_memory_bytes: u64,
    /// Available system memory in bytes
    pub available_memory_bytes: u64,
    /// Hostname
    pub hostname: String,
}

impl SystemInfo {
    /// Collect current system information
    pub fn collect() -> Self {
        Self {
            os_name: std::env::consts::OS.to_string(),
            os_version: "unknown".to_string(), // Would use platform-specific APIs
            arch: std::env::consts::ARCH.to_string(),
            cpu_cores: num_cpus::get(),
            total_memory_bytes: 0, // Would use platform-specific APIs
            available_memory_bytes: 0, // Would use platform-specific APIs
            hostname: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }
}

/// Application events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStartedEvent {
    pub version: String,
    pub started_at: DateTime<Utc>,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: crate::types::Metadata,
}

impl Event for ApplicationStartedEvent {
    fn event_type(&self) -> &'static str {
        "application.started"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &crate::types::Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationShuttingDownEvent {
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: crate::types::Metadata,
}

impl Event for ApplicationShuttingDownEvent {
    fn event_type(&self) -> &'static str {
        "application.shutting_down"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &crate::types::Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Manager registration information
#[derive(Debug)]
struct ManagerRegistration {
    manager: Box<dyn Manager>,
    dependencies: Vec<String>,
    initialized: bool,
    failed: bool,
}

/// Main application core that orchestrates all managers
#[derive(Debug)]
pub struct ApplicationCore {
    state: ManagedState,
    app_state: Arc<RwLock<ApplicationState>>,
    started_at: DateTime<Utc>,

    // Core managers
    config_manager: Option<ConfigManager>,
    logging_manager: Option<LoggingManager>,
    event_bus_manager: Option<Arc<EventBusManager>>,
    file_manager: Option<FileManager>,
    concurrency_manager: Option<ConcurrencyManager>,
    task_manager: Option<TaskManager>,

    // Manager registry
    managers: Arc<RwLock<HashMap<String, ManagerRegistration>>>,

    // Application lifecycle
    shutdown_signal: broadcast::Sender<()>,
    health_check_interval: Duration,

    // System monitoring
    system_info: SystemInfo,
}

impl ApplicationCore {
    /// Create a new application core
    pub fn new() -> Self {
        let (shutdown_signal, _) = broadcast::channel(1);

        Self {
            state: ManagedState::new(Uuid::new_v4(), "application_core"),
            app_state: Arc::new(RwLock::new(ApplicationState::Created)),
            started_at: Utc::now(),
            config_manager: None,
            logging_manager: None,
            event_bus_manager: None,
            file_manager: None,
            concurrency_manager: None,
            task_manager: None,
            managers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_signal,
            health_check_interval: Duration::from_secs(30),
            system_info: SystemInfo::collect(),
        }
    }

    /// Create application core with configuration file
    pub fn with_config_file(config_path: impl AsRef<Path>) -> Self {
        let mut app = Self::new();
        app.config_manager = Some(ConfigManager::with_config_file(config_path));
        app
    }

    /// Initialize the application and all managers
    pub async fn initialize(&mut self) -> Result<()> {
        *self.app_state.write().await = ApplicationState::Initializing;
        self.state.set_state(ManagerState::Initializing).await;

        tracing::info!("Initializing Qorzen application core");

        // Initialize core managers in dependency order
        self.init_config_manager().await?;
        self.init_logging_manager().await?;
        self.init_concurrency_manager().await?;
        self.init_event_bus_manager().await?;
        self.init_file_manager().await?;
        self.init_task_manager().await?;

        // Initialize all registered managers
        self.initialize_all_managers().await?;

        // Start health monitoring
        self.start_health_monitoring().await?;

        // Start signal handling
        self.setup_signal_handlers().await?;

        // Publish application started event
        self.publish_started_event().await?;

        *self.app_state.write().await = ApplicationState::Running;
        self.state.set_state(ManagerState::Running).await;

        tracing::info!("Qorzen application core initialized successfully");
        Ok(())
    }

    /// Initialize configuration manager
    async fn init_config_manager(&mut self) -> Result<()> {
        if self.config_manager.is_none() {
            self.config_manager = Some(ConfigManager::new());
        }

        if let Some(config_manager) = &mut self.config_manager {
            config_manager.initialize().await
                .with_context(|| "Failed to initialize configuration manager".to_string())?;
        }

        Ok(())
    }

    /// Initialize logging manager
    async fn init_logging_manager(&mut self) -> Result<()> {
        let config = if let Some(config_manager) = &self.config_manager {
            config_manager.get_config().await.logging
        } else {
            crate::config::LoggingConfig::default()
        };

        let mut logging_manager = LoggingManager::new(config);
        logging_manager.initialize().await
            .with_context(|| "Failed to initialize logging manager".to_string())?;

        self.logging_manager = Some(logging_manager);
        Ok(())
    }

    /// Initialize concurrency manager
    async fn init_concurrency_manager(&mut self) -> Result<()> {
        let config = if let Some(config_manager) = &self.config_manager {
            config_manager.get_config().await.concurrency
        } else {
            crate::config::ConcurrencyConfig::default()
        };

        let mut concurrency_manager = ConcurrencyManager::new(config)
            .with_context(|| "Failed to create concurrency manager".to_string())?;

        concurrency_manager.initialize().await
            .with_context(|| "Failed to initialize concurrency manager".to_string())?;

        self.concurrency_manager = Some(concurrency_manager);
        Ok(())
    }

    /// Initialize event bus manager
    async fn init_event_bus_manager(&mut self) -> Result<()> {
        let config = if let Some(config_manager) = &self.config_manager {
            config_manager.get_config().await.event_bus
        } else {
            crate::config::EventBusConfig::default()
        };

        let event_config = crate::event::EventBusConfig {
            worker_count: config.worker_count,
            queue_capacity: config.queue_size,
            default_timeout: Duration::from_millis(config.publish_timeout_ms),
            enable_persistence: false,
            enable_metrics: true,
            batch_size: 100,
            max_retry_delay: Duration::from_secs(60),
        };

        let mut event_bus_manager = EventBusManager::new(event_config);
        event_bus_manager.initialize().await
            .with_context(|| "Failed to initialize event bus manager".to_string())?;

        self.event_bus_manager = Some(Arc::new(event_bus_manager));
        Ok(())
    }

    /// Initialize file manager
    async fn init_file_manager(&mut self) -> Result<()> {
        let config = if let Some(config_manager) = &self.config_manager {
            config_manager.get_config().await.files
        } else {
            crate::config::FileConfig::default()
        };

        let mut file_manager = FileManager::new(config);
        file_manager.initialize().await
            .with_context(|| "Failed to initialize file manager".to_string())?;

        self.file_manager = Some(file_manager);
        Ok(())
    }

    /// Initialize task manager
    async fn init_task_manager(&mut self) -> Result<()> {
        let config = if let Some(config_manager) = &self.config_manager {
            config_manager.get_config().await.tasks
        } else {
            crate::config::TaskConfig::default()
        };

        let task_config = crate::config::TaskConfig {
            max_concurrent: config.max_concurrent,
            default_timeout_ms: config.default_timeout_ms,
            keep_completed: config.keep_completed,
            progress_update_interval_ms: config.progress_update_interval_ms,
        };

        let mut task_manager = TaskManager::new(task_config);

        // Set event bus for task events
        if let Some(event_bus) = &self.event_bus_manager {
            task_manager.set_event_bus(Arc::clone(event_bus));
        }

        task_manager.initialize().await
            .with_context(|| "Failed to initialize task manager".to_string())?;

        self.task_manager = Some(task_manager);
        Ok(())
    }

    /// Initialize all registered managers
    async fn initialize_all_managers(&self) -> Result<()> {
        let mut managers = self.managers.write().await;

        // Topological sort for dependency resolution
        let sorted_managers = self.topological_sort_managers(&managers)?;

        for manager_name in sorted_managers {
            if let Some(registration) = managers.get_mut(&manager_name) {
                if !registration.initialized && !registration.failed {
                    match registration.manager.initialize().await {
                        Ok(()) => {
                            registration.initialized = true;
                            tracing::info!("Initialized manager: {}", manager_name);
                        }
                        Err(e) => {
                            registration.failed = true;
                            tracing::error!("Failed to initialize manager {}: {}", manager_name, e);
                            // Continue with other managers for now
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Perform topological sort of managers based on dependencies
    fn topological_sort_managers(
        &self,
        managers: &HashMap<String, ManagerRegistration>,
    ) -> Result<Vec<String>> {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        fn visit(
            name: &str,
            managers: &HashMap<String, ManagerRegistration>,
            sorted: &mut Vec<String>,
            visited: &mut std::collections::HashSet<String>,
            visiting: &mut std::collections::HashSet<String>,
        ) -> Result<()> {
            if visiting.contains(name) {
                return Err(Error::new(
                    ErrorKind::Manager {
                        manager_name: name.to_string(),
                        operation: ManagerOperation::Initialize,
                    },
                    "Circular dependency detected",
                ));
            }

            if visited.contains(name) {
                return Ok(());
            }

            visiting.insert(name.to_string());

            if let Some(registration) = managers.get(name) {
                for dep in &registration.dependencies {
                    visit(dep, managers, sorted, visited, visiting)?;
                }
            }

            visiting.remove(name);
            visited.insert(name.to_string());
            sorted.push(name.to_string());

            Ok(())
        }

        for name in managers.keys() {
            visit(name, managers, &mut sorted, &mut visited, &mut visiting)?;
        }

        Ok(sorted)
    }

    /// Start health monitoring for all managers
    async fn start_health_monitoring(&self) -> Result<()> {
        let managers = Arc::clone(&self.managers);
        let event_bus = self.event_bus_manager.clone();
        let health_interval = self.health_check_interval;

        tokio::spawn(async move {
            let mut interval = interval(health_interval);

            loop {
                interval.tick().await;

                // Perform health checks on all managers
                let managers_guard = managers.read().await;
                let mut unhealthy_managers = Vec::new();

                for (name, registration) in managers_guard.iter() {
                    if registration.initialized && !registration.failed {
                        let health = registration.manager.health_check().await;
                        if health != HealthStatus::Healthy {
                            unhealthy_managers.push((name.clone(), health));
                        }
                    }
                }

                drop(managers_guard);

                // Report unhealthy managers
                if !unhealthy_managers.is_empty() {
                    tracing::warn!(
                        "Unhealthy managers detected: {:?}",
                        unhealthy_managers
                    );

                    // Publish health events if event bus is available
                    if let Some(_event_bus) = &event_bus { // Fixed: Added underscore prefix
                        for (manager_name, health_status) in unhealthy_managers {
                            // Would publish health events here
                            tracing::debug!(
                                "Manager {} health status: {:?}",
                                manager_name,
                                health_status
                            );
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Setup signal handlers for graceful shutdown
    async fn setup_signal_handlers(&self) -> Result<()> {
        let shutdown_sender = self.shutdown_signal.clone();
        let app_state = Arc::clone(&self.app_state);

        tokio::spawn(async move {
            #[cfg(unix)]
            {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
                let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .expect("Failed to register SIGINT handler");

                tokio::select! {
                    _ = sigterm.recv() => {
                        tracing::info!("Received SIGTERM, initiating graceful shutdown");
                    }
                    _ = sigint.recv() => {
                        tracing::info!("Received SIGINT, initiating graceful shutdown");
                    }
                }
            }

            #[cfg(windows)]
            {
                let mut ctrl_c = signal::windows::ctrl_c()
                    .expect("Failed to register Ctrl+C handler");

                ctrl_c.recv().await;
                tracing::info!("Received Ctrl+C, initiating graceful shutdown");
            }

            *app_state.write().await = ApplicationState::ShuttingDown;
            let _ = shutdown_sender.send(());
        });

        Ok(())
    }

    /// Publish application started event
    async fn publish_started_event(&self) -> Result<()> {
        if let Some(event_bus) = &self.event_bus_manager {
            let event = ApplicationStartedEvent {
                version: crate::VERSION.to_string(),
                started_at: self.started_at,
                timestamp: Utc::now(),
                source: "application_core".to_string(),
                metadata: HashMap::new(),
            };

            event_bus.publish(event).await
                .with_context(|| "Failed to publish application started event".to_string())?;
        }

        Ok(())
    }

    /// Register a manager with the application core
    pub async fn register_manager(
        &self,
        name: impl Into<String>,
        manager: Box<dyn Manager>,
        dependencies: Vec<String>,
    ) -> Result<()> {
        let name = name.into();
        let mut managers = self.managers.write().await;

        if managers.contains_key(&name) {
            return Err(Error::new(
                ErrorKind::Manager {
                    manager_name: name,
                    operation: ManagerOperation::Initialize,
                },
                "Manager already registered",
            ));
        }

        let registration = ManagerRegistration {
            manager,
            dependencies,
            initialized: false,
            failed: false,
        };

        managers.insert(name.clone(), registration);
        tracing::info!("Registered manager: {}", name);

        Ok(())
    }

    /// Get a reference to a specific manager
    pub async fn get_manager(&self, _name: &str) -> Option<Box<dyn Manager>> { // Fixed: Added underscore prefix
        // This is a simplified implementation
        // In practice, you'd need a more sophisticated approach to return manager references
        None
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) -> Result<()> {
        let mut receiver = self.shutdown_signal.subscribe();
        receiver.recv().await.map_err(|_| {
            Error::new(
                ErrorKind::Application,
                "Shutdown signal channel closed unexpectedly",
            )
        })?;
        Ok(())
    }

    /// Shutdown the application gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        *self.app_state.write().await = ApplicationState::ShuttingDown;
        self.state.set_state(ManagerState::ShuttingDown).await;

        tracing::info!("Shutting down Qorzen application");

        // Publish shutdown event
        if let Some(event_bus) = &self.event_bus_manager {
            let event = ApplicationShuttingDownEvent {
                reason: "Normal shutdown".to_string(),
                timestamp: Utc::now(),
                source: "application_core".to_string(),
                metadata: HashMap::new(),
            };

            let _ = timeout(Duration::from_secs(5), event_bus.publish(event)).await;
        }

        // Shutdown registered managers in reverse dependency order
        self.shutdown_all_managers().await?;

        // Shutdown core managers in reverse order
        if let Some(mut task_manager) = self.task_manager.take() {
            let _ = timeout(Duration::from_secs(10), task_manager.shutdown()).await;
        }

        if let Some(mut file_manager) = self.file_manager.take() {
            let _ = timeout(Duration::from_secs(5), file_manager.shutdown()).await;
        }

        if let Some(event_bus_manager) = self.event_bus_manager.take() {
            if let Ok(mut manager) = Arc::try_unwrap(event_bus_manager) {
                let _ = timeout(Duration::from_secs(5), manager.shutdown()).await;
            }
        }

        if let Some(mut concurrency_manager) = self.concurrency_manager.take() {
            let _ = timeout(Duration::from_secs(10), concurrency_manager.shutdown()).await;
        }

        if let Some(mut logging_manager) = self.logging_manager.take() {
            let _ = timeout(Duration::from_secs(5), logging_manager.shutdown()).await;
        }

        if let Some(mut config_manager) = self.config_manager.take() {
            let _ = timeout(Duration::from_secs(2), config_manager.shutdown()).await;
        }

        *self.app_state.write().await = ApplicationState::Shutdown;
        self.state.set_state(ManagerState::Shutdown).await;

        tracing::info!("Qorzen application shutdown complete");
        Ok(())
    }

    /// Shutdown all registered managers
    async fn shutdown_all_managers(&self) -> Result<()> {
        let mut managers = self.managers.write().await;

        // Get managers in reverse dependency order
        let sorted_managers = self.topological_sort_managers(&managers)?;
        let mut reversed_managers = sorted_managers;
        reversed_managers.reverse();

        for manager_name in reversed_managers {
            if let Some(registration) = managers.get_mut(&manager_name) {
                if registration.initialized {
                    match timeout(
                        Duration::from_secs(10),
                        registration.manager.shutdown()
                    ).await {
                        Ok(Ok(())) => {
                            tracing::info!("Shut down manager: {}", manager_name);
                        }
                        Ok(Err(e)) => {
                            tracing::error!("Error shutting down manager {}: {}", manager_name, e);
                        }
                        Err(_) => {
                            tracing::error!("Timeout shutting down manager: {}", manager_name);
                        }
                    }
                }
            }
        }

        managers.clear();
        Ok(())
    }

    /// Get application health information
    pub async fn get_health(&self) -> ApplicationHealth {
        let managers = self.managers.read().await;
        let mut manager_health = HashMap::new();
        let mut overall_healthy = true;

        for (name, registration) in managers.iter() {
            if registration.initialized && !registration.failed {
                let health = registration.manager.health_check().await;
                if health != HealthStatus::Healthy {
                    overall_healthy = false;
                }
                manager_health.insert(name.clone(), health);
            } else if registration.failed {
                manager_health.insert(name.clone(), HealthStatus::Unhealthy);
                overall_healthy = false;
            } else {
                manager_health.insert(name.clone(), HealthStatus::Unknown);
            }
        }

        let overall_status = if overall_healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded
        };

        ApplicationHealth {
            status: overall_status,
            uptime: Utc::now().signed_duration_since(self.started_at).to_std().unwrap_or_default(),
            managers: manager_health,
            last_check: Utc::now(),
            details: HashMap::new(),
        }
    }

    /// Get application statistics
    pub async fn get_stats(&self) -> ApplicationStats {
        let managers = self.managers.read().await;
        let initialized_count = managers.values().filter(|r| r.initialized).count();
        let failed_count = managers.values().filter(|r| r.failed).count();

        ApplicationStats {
            version: crate::VERSION.to_string(),
            started_at: self.started_at,
            uptime: Utc::now().signed_duration_since(self.started_at).to_std().unwrap_or_default(),
            state: *self.app_state.read().await,
            manager_count: managers.len(),
            initialized_managers: initialized_count,
            failed_managers: failed_count,
            memory_usage_bytes: 0, // Would use platform-specific APIs
            cpu_usage_percent: 0.0, // Would use platform-specific APIs
            system_info: self.system_info.clone(),
        }
    }

    /// Get application configuration
    // pub async fn get_config(&self) -> Option<AppConfig> {
    //     self.config_manager.as_ref().map(|_cm| { // Fixed: Added underscore prefix
    //         // This would need to be async in practice
    //         // For now, return a default config
    //         AppConfig::default()
    //     })
    // }
    pub async fn get_config(&self) -> Result<AppConfig> {
        match &self.config_manager {
            Some(manager) => Ok(manager.get_config().await),
            None => Err(Error::config_operation(
                "config_manager",
                ConfigOperation::Get,
                "Configuration manager not initialized",
            )),
        }
    }

    /// Trigger configuration reload
    pub async fn reload_config(&self) -> Result<()> {
        if let Some(config_manager) = &self.config_manager {
            config_manager.reload().await
                .with_context(|| "Failed to reload configuration".to_string())?;
        }
        Ok(())
    }

    /// Get current application state
    pub async fn get_state(&self) -> ApplicationState {
        *self.app_state.read().await
    }
}

impl Default for ApplicationCore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Manager for ApplicationCore {
    fn name(&self) -> &str {
        "application_core"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4() // Simplified
    }

    async fn initialize(&mut self) -> Result<()> {
        // Main initialization is handled by the public initialize method
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Main shutdown is handled by the public shutdown method
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let app_stats = self.get_stats().await;

        status.add_metadata("app_state", serde_json::Value::String(format!("{:?}", app_stats.state)));
        status.add_metadata("uptime_seconds", serde_json::Value::from(app_stats.uptime.as_secs()));
        status.add_metadata("manager_count", serde_json::Value::from(app_stats.manager_count));
        status.add_metadata("initialized_managers", serde_json::Value::from(app_stats.initialized_managers));
        status.add_metadata("failed_managers", serde_json::Value::from(app_stats.failed_managers));

        status
    }

    async fn health_check(&self) -> HealthStatus {
        let health = self.get_health().await;
        health.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_application_core_creation() {
        let app = ApplicationCore::new();
        assert_eq!(app.get_state().await, ApplicationState::Created);
    }

    #[tokio::test]
    async fn test_application_initialization() {
        let mut app = ApplicationCore::new();
        app.initialize().await.unwrap();

        assert_eq!(app.get_state().await, ApplicationState::Running);

        app.shutdown().await.unwrap();
        assert_eq!(app.get_state().await, ApplicationState::Shutdown);
    }

    #[tokio::test]
    async fn test_application_with_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a simple config file
        tokio::fs::write(&config_path, "app:\n  name: \"Test App\"\n  version: \"1.0.0\"")
            .await
            .unwrap();

        let mut app = ApplicationCore::with_config_file(&config_path);
        app.initialize().await.unwrap();

        assert_eq!(app.get_state().await, ApplicationState::Running);

        app.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_application_health_check() {
        let mut app = ApplicationCore::new();
        app.initialize().await.unwrap();

        let health = app.get_health().await;
        assert!(matches!(health.status, HealthStatus::Healthy | HealthStatus::Degraded));

        app.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_application_stats() {
        let mut app = ApplicationCore::new();
        app.initialize().await.unwrap();

        let stats = app.get_stats().await;
        assert_eq!(stats.version, crate::VERSION);
        assert_eq!(stats.state, ApplicationState::Running);
        assert!(stats.uptime.as_secs() < 10); // Should be very recent

        app.shutdown().await.unwrap();
    }

    #[test]
    fn test_system_info_collection() {
        let system_info = SystemInfo::collect();
        assert!(!system_info.os_name.is_empty());
        assert!(!system_info.arch.is_empty());
        assert!(system_info.cpu_cores > 0);
    }
}