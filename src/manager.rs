// src/manager.rs

//! Manager system for coordinating application components
//!
//! This module provides the foundational manager infrastructure that allows
//! different system components to be managed in a coordinated way. Features include:
//! - Unified lifecycle management (initialize, run, shutdown)
//! - Health monitoring and status reporting
//! - Dependency resolution between managers
//! - Graceful error handling and recovery
//! - State management and transitions
//! - Event-driven communication between managers

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{Error, ManagerOperation, Result};
use crate::types::Metadata;

/// Manager state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagerState {
    /// Manager has been created but not initialized
    Created,
    /// Manager is currently initializing
    Initializing,
    /// Manager is running normally
    Running,
    /// Manager is paused (can be resumed)
    Paused,
    /// Manager is shutting down
    ShuttingDown,
    /// Manager has shut down
    Shutdown,
    /// Manager is in an error state
    Error,
    /// Manager is in maintenance mode
    Maintenance,
}

impl fmt::Display for ManagerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "CREATED"),
            Self::Initializing => write!(f, "INITIALIZING"),
            Self::Running => write!(f, "RUNNING"),
            Self::Paused => write!(f, "PAUSED"),
            Self::ShuttingDown => write!(f, "SHUTTING_DOWN"),
            Self::Shutdown => write!(f, "SHUTDOWN"),
            Self::Error => write!(f, "ERROR"),
            Self::Maintenance => write!(f, "MAINTENANCE"),
        }
    }
}

/// Health status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Manager is healthy and operating normally
    Healthy,
    /// Manager is operational but with some issues
    Degraded,
    /// Manager is not functioning properly
    Unhealthy,
    /// Health status is unknown
    Unknown,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "HEALTHY"),
            Self::Degraded => write!(f, "DEGRADED"),
            Self::Unhealthy => write!(f, "UNHEALTHY"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Manager status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    /// Manager unique identifier
    pub id: Uuid,
    /// Manager name
    pub name: String,
    /// Current state
    pub state: ManagerState,
    /// Health status
    pub health: HealthStatus,
    /// When the manager was created
    pub created_at: DateTime<Utc>,
    /// When the manager was last started
    pub started_at: Option<DateTime<Utc>>,
    /// Manager uptime
    pub uptime: Option<Duration>,
    /// Last status update time
    pub last_updated: DateTime<Utc>,
    /// Status message
    pub message: Option<String>,
    /// Additional metadata
    pub metadata: Metadata,
    /// Performance metrics
    pub metrics: ManagerMetrics,
}

impl ManagerStatus {
    /// Create a new manager status
    pub fn new(id: Uuid, name: impl Into<String>, state: ManagerState) -> Self {
        Self {
            id,
            name: name.into(),
            state,
            health: HealthStatus::Unknown,
            created_at: Utc::now(),
            started_at: None,
            uptime: None,
            last_updated: Utc::now(),
            message: None,
            metadata: HashMap::new(),
            metrics: ManagerMetrics::default(),
        }
    }

    /// Update the status
    pub fn update_state(&mut self, state: ManagerState) {
        self.state = state;
        self.last_updated = Utc::now();

        // Set started_at when transitioning to running
        if state == ManagerState::Running && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }

        // Calculate uptime
        if let Some(started) = self.started_at {
            if matches!(state, ManagerState::Running | ManagerState::Paused) {
                self.uptime = Utc::now()
                    .signed_duration_since(started)
                    .to_std()
                    .ok();
            }
        }
    }

    /// Set health status
    pub fn set_health(&mut self, health: HealthStatus) {
        self.health = health;
        self.last_updated = Utc::now();
    }

    /// Set status message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.last_updated = Utc::now();
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
        self.last_updated = Utc::now();
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: ManagerMetrics) {
        self.metrics = metrics;
        self.last_updated = Utc::now();
    }
}

/// Manager performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerMetrics {
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Number of active tasks/operations
    pub active_operations: u32,
    /// Total operations processed
    pub total_operations: u64,
    /// Operations per second
    pub operations_per_second: f64,
    /// Average operation latency in milliseconds
    pub avg_latency_ms: f64,
    /// Error rate (errors per total operations)
    pub error_rate: f64,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for ManagerMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            active_operations: 0,
            total_operations: 0,
            operations_per_second: 0.0,
            avg_latency_ms: 0.0,
            error_rate: 0.0,
            custom_metrics: HashMap::new(),
        }
    }
}

/// Manager trait that all system managers must implement
#[async_trait]
pub trait Manager: Send + Sync + fmt::Debug {
    /// Get the manager name
    fn name(&self) -> &str;

    /// Get the manager unique identifier
    fn id(&self) -> Uuid;

    /// Initialize the manager
    async fn initialize(&mut self) -> Result<()>;

    /// Shutdown the manager gracefully
    async fn shutdown(&mut self) -> Result<()>;

    /// Get current manager status
    async fn status(&self) -> ManagerStatus;

    /// Perform health check
    async fn health_check(&self) -> HealthStatus {
        // Default implementation based on current state
        let status = self.status().await;
        match status.state {
            ManagerState::Running => HealthStatus::Healthy,
            ManagerState::Paused | ManagerState::Maintenance => HealthStatus::Degraded,
            ManagerState::Error => HealthStatus::Unhealthy,
            _ => HealthStatus::Unknown,
        }
    }

    /// Pause the manager (if supported)
    async fn pause(&mut self) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Pause,
            "Pause operation not supported",
        ))
    }

    /// Resume the manager from paused state (if supported)
    async fn resume(&mut self) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Resume,
            "Resume operation not supported",
        ))
    }

    /// Restart the manager
    async fn restart(&mut self) -> Result<()> {
        self.shutdown().await?;
        self.initialize().await
    }

    /// Get manager configuration (if any)
    async fn get_config(&self) -> Option<serde_json::Value> {
        None
    }

    /// Update manager configuration (if supported)
    async fn update_config(&mut self, _config: serde_json::Value) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Configure,
            "Configuration update not supported",
        ))
    }

    /// Get manager dependencies
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    /// Get manager priority (for initialization order)
    fn priority(&self) -> i32 {
        0
    }

    /// Whether this manager is essential (system fails if this manager fails)
    fn is_essential(&self) -> bool {
        false
    }

    /// Get manager version
    fn version(&self) -> Option<String> {
        None
    }

    /// Get manager description
    fn description(&self) -> Option<String> {
        None
    }
}

/// Managed state helper for implementing managers
pub struct ManagedState {
    id: Uuid,
    name: String,
    status: Arc<RwLock<ManagerStatus>>,
}

impl ManagedState {
    /// Create a new managed state
    pub fn new(id: Uuid, name: impl Into<String>) -> Self {
        let name_str = name.into();
        let status = ManagerStatus::new(id, name_str.clone(), ManagerState::Created);

        Self {
            id,
            name: name_str,
            status: Arc::new(RwLock::new(status)),
        }
    }

    /// Get the manager ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the manager name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the manager state
    pub async fn set_state(&self, state: ManagerState) {
        let mut status = self.status.write().await;
        status.update_state(state);
    }

    /// Set health status
    pub async fn set_health(&self, health: HealthStatus) {
        let mut status = self.status.write().await;
        status.set_health(health);
    }

    /// Set status message
    pub async fn set_message(&self, message: impl Into<String>) {
        let mut status = self.status.write().await;
        status.set_message(message);
    }

    /// Add metadata
    pub async fn add_metadata(&self, key: impl Into<String>, value: serde_json::Value) {
        let mut status = self.status.write().await;
        status.add_metadata(key, value);
    }

    /// Update metrics
    pub async fn update_metrics(&self, metrics: ManagerMetrics) {
        let mut status = self.status.write().await;
        status.update_metrics(metrics);
    }

    /// Get current status
    pub async fn status(&self) -> ManagerStatus {
        self.status.read().await.clone()
    }

    /// Get current state
    pub async fn state(&self) -> ManagerState {
        self.status.read().await.state
    }

    /// Get current health
    pub async fn health(&self) -> HealthStatus {
        self.status.read().await.health
    }
}

impl fmt::Debug for ManagedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManagedState")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

/// Manager registry for managing multiple managers
#[derive(Debug)]
pub struct ManagerRegistry {
    managers: HashMap<String, Box<dyn Manager>>,
    dependencies: HashMap<String, Vec<String>>,
    initialization_order: Vec<String>,
}

impl ManagerRegistry {
    /// Create a new manager registry
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
            dependencies: HashMap::new(),
            initialization_order: Vec::new(),
        }
    }

    /// Register a manager
    pub fn register<M: Manager + 'static>(
        &mut self,
        manager: M,
        dependencies: Vec<String>,
    ) -> Result<()> {
        let name = manager.name().to_string();

        if self.managers.contains_key(&name) {
            return Err(Error::manager(
                &name,
                ManagerOperation::Register,
                "Manager already registered",
            ));
        }

        self.managers.insert(name.clone(), Box::new(manager));
        self.dependencies.insert(name.clone(), dependencies);

        // Recalculate initialization order
        self.calculate_initialization_order()?;

        Ok(())
    }

    /// Unregister a manager
    pub fn unregister(&mut self, name: &str) -> Result<Box<dyn Manager>> {
        let manager = self.managers.remove(name).ok_or_else(|| {
            Error::manager(name, ManagerOperation::Unregister, "Manager not found")
        })?;

        self.dependencies.remove(name);
        self.initialization_order.retain(|n| n != name);

        Ok(manager)
    }

    /// Get a manager by name
    pub fn get(&self, name: &str) -> Option<&dyn Manager> {
        self.managers.get(name).map(|m| m.as_ref())
    }

    /// Get a mutable reference to a manager with explicit lifetime bounds
    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn Manager> {
        let entry = self.managers.get_mut(name)?;
        Some(entry.as_mut())
    }

    /// Initialize all managers in dependency order
    pub async fn initialize_all(&mut self) -> Result<()> {
        for name in &self.initialization_order.clone() {
            if let Some(manager) = self.managers.get_mut(name) {
                manager.initialize().await.map_err(|e| {
                    Error::manager(
                        name,
                        ManagerOperation::Initialize,
                        format!("Failed to initialize manager: {}", e),
                    )
                })?;
            }
        }
        Ok(())
    }

    /// Shutdown all managers in reverse dependency order
    pub async fn shutdown_all(&mut self) -> Result<()> {
        let mut shutdown_order = self.initialization_order.clone();
        shutdown_order.reverse();

        for name in &shutdown_order {
            if let Some(manager) = self.managers.get_mut(name) {
                if let Err(e) = manager.shutdown().await {
                    tracing::error!("Failed to shutdown manager {}: {}", name, e);
                    // Continue with other managers
                }
            }
        }
        Ok(())
    }

    /// Get status of all managers
    pub async fn get_all_status(&self) -> HashMap<String, ManagerStatus> {
        let mut status_map = HashMap::new();

        for (name, manager) in &self.managers {
            status_map.insert(name.clone(), manager.status().await);
        }

        status_map
    }

    /// Perform health check on all managers
    pub async fn health_check_all(&self) -> HashMap<String, HealthStatus> {
        let mut health_map = HashMap::new();

        for (name, manager) in &self.managers {
            health_map.insert(name.clone(), manager.health_check().await);
        }

        health_map
    }

    /// Calculate initialization order based on dependencies
    fn calculate_initialization_order(&mut self) -> Result<()> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for name in self.managers.keys() {
            if !visited.contains(name) {
                self.visit_manager(name, &mut order, &mut visited, &mut visiting)?;
            }
        }

        self.initialization_order = order;
        Ok(())
    }

    /// Depth-first search for topological sorting
    fn visit_manager(
        &self,
        name: &str,
        order: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visiting.contains(name) {
            return Err(Error::manager(
                name,
                ManagerOperation::Initialize,
                "Circular dependency detected",
            ));
        }

        if visited.contains(name) {
            return Ok(());
        }

        visiting.insert(name.to_string());

        if let Some(deps) = self.dependencies.get(name) {
            for dep in deps {
                if !self.managers.contains_key(dep) {
                    return Err(Error::manager(
                        name,
                        ManagerOperation::Initialize,
                        format!("Dependency '{}' not found", dep),
                    ));
                }
                self.visit_manager(dep, order, visited, visiting)?;
            }
        }

        visiting.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());

        Ok(())
    }

    /// Get initialization order
    pub fn get_initialization_order(&self) -> &[String] {
        &self.initialization_order
    }

    /// Get manager count
    pub fn count(&self) -> usize {
        self.managers.len()
    }

    /// List all manager names
    pub fn list_names(&self) -> Vec<String> {
        self.managers.keys().cloned().collect()
    }
}

impl Default for ManagerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager collection for easier bulk operations
#[derive(Debug)]
pub struct ManagerCollection {
    registry: ManagerRegistry,
    parallel_operations: bool,
    operation_timeout: Duration,
}

impl ManagerCollection {
    /// Create a new manager collection
    pub fn new() -> Self {
        Self {
            registry: ManagerRegistry::new(),
            parallel_operations: false,
            operation_timeout: Duration::from_secs(30),
        }
    }

    /// Enable parallel operations where possible
    pub fn with_parallel_operations(mut self, enabled: bool) -> Self {
        self.parallel_operations = enabled;
        self
    }

    /// Set operation timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    /// Add a manager to the collection
    pub fn add<M: Manager + 'static>(
        &mut self,
        manager: M,
        dependencies: Vec<String>,
    ) -> Result<()> {
        self.registry.register(manager, dependencies)
    }

    /// Initialize all managers
    pub async fn initialize_all(&mut self) -> Result<()> {
        if self.parallel_operations {
            // Parallel initialization (respecting dependencies)
            self.initialize_parallel().await
        } else {
            // Sequential initialization
            self.registry.initialize_all().await
        }
    }

    /// Shutdown all managers
    pub async fn shutdown_all(&mut self) -> Result<()> {
        self.registry.shutdown_all().await
    }

    /// Parallel initialization respecting dependencies
    async fn initialize_parallel(&mut self) -> Result<()> {
        // This is a simplified implementation
        // In practice, you'd need a more sophisticated approach
        // to handle parallel initialization with dependencies
        self.registry.initialize_all().await
    }

    /// Get registry reference
    pub fn registry(&self) -> &ManagerRegistry {
        &self.registry
    }

    /// Get mutable registry reference
    pub fn registry_mut(&mut self) -> &mut ManagerRegistry {
        &mut self.registry
    }
}

impl Default for ManagerCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock manager for testing
    #[derive(Debug)]
    struct MockManager {
        name: String,
        state: ManagedState,
        should_fail: bool,
    }

    impl MockManager {
        fn new(name: impl Into<String>) -> Self {
            let name_str = name.into();
            Self {
                state: ManagedState::new(Uuid::new_v4(), name_str.clone()),
                name: name_str,
                should_fail: false,
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    #[async_trait]
    impl Manager for MockManager {
        fn name(&self) -> &str {
            &self.name
        }

        fn id(&self) -> Uuid {
            self.state.id()
        }

        async fn initialize(&mut self) -> Result<()> {
            if self.should_fail {
                return Err(Error::manager(
                    &self.name,
                    ManagerOperation::Initialize,
                    "Mock initialization failure",
                ));
            }

            self.state.set_state(ManagerState::Running).await;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            self.state.set_state(ManagerState::Shutdown).await;
            Ok(())
        }

        async fn status(&self) -> ManagerStatus {
            self.state.status().await
        }
    }

    #[tokio::test]
    async fn test_managed_state() {
        let state = ManagedState::new(Uuid::new_v4(), "test_manager");

        assert_eq!(state.name(), "test_manager");
        assert_eq!(state.state().await, ManagerState::Created);

        state.set_state(ManagerState::Running).await;
        assert_eq!(state.state().await, ManagerState::Running);

        state.set_health(HealthStatus::Healthy).await;
        assert_eq!(state.health().await, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_manager_registry() {
        let mut registry = ManagerRegistry::new();

        let manager1 = MockManager::new("manager1");
        let manager2 = MockManager::new("manager2");

        // Register managers
        registry
            .register(manager1, vec![])
            .unwrap();
        registry
            .register(manager2, vec!["manager1".to_string()])
            .unwrap();

        assert_eq!(registry.count(), 2);

        // Check initialization order
        let order = registry.get_initialization_order();
        assert_eq!(order, &["manager1", "manager2"]);

        // Initialize all
        registry.initialize_all().await.unwrap();

        // Check status
        let status_map = registry.get_all_status().await;
        assert_eq!(status_map.len(), 2);
        assert_eq!(status_map["manager1"].state, ManagerState::Running);
        assert_eq!(status_map["manager2"].state, ManagerState::Running);

        // Shutdown all
        registry.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let mut registry = ManagerRegistry::new();

        let manager1 = MockManager::new("manager1");
        let manager2 = MockManager::new("manager2");

        registry
            .register(manager1, vec!["manager2".to_string()])
            .unwrap();

        let result = registry.register(manager2, vec!["manager1".to_string()]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_collection() {
        let mut collection = ManagerCollection::new();

        let manager = MockManager::new("test_manager");
        collection.add(manager, vec![]).unwrap();

        collection.initialize_all().await.unwrap();
        collection.shutdown_all().await.unwrap();
    }

    #[test]
    fn test_manager_state_display() {
        assert_eq!(ManagerState::Running.to_string(), "RUNNING");
        assert_eq!(ManagerState::Error.to_string(), "ERROR");
        assert_eq!(HealthStatus::Healthy.to_string(), "HEALTHY");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "UNHEALTHY");
    }
}