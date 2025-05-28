// src/manager.rs - Enhanced manager system with plugin support

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, ManagerOperation, Result};
use crate::types::Metadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagerState {
    Created,
    Initializing,
    Running,
    Paused,
    ShuttingDown,
    Shutdown,
    Error,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRequirements {
    pub requires_filesystem: bool,
    pub requires_network: bool,
    pub requires_database: bool,
    pub requires_native_apis: bool,
    pub minimum_permissions: Vec<String>,
}

impl Default for PlatformRequirements {
    fn default() -> Self {
        Self {
            requires_filesystem: false,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    pub id: Uuid,
    pub name: String,
    pub state: ManagerState,
    pub health: HealthStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub uptime: Option<Duration>,
    pub last_updated: DateTime<Utc>,
    pub message: Option<String>,
    pub metadata: Metadata,
    pub metrics: ManagerMetrics,
}

impl ManagerStatus {
    /// Creates a new manager status
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

    /// Updates the manager state
    pub fn update_state(&mut self, state: ManagerState) {
        self.state = state;
        self.last_updated = Utc::now();

        if state == ManagerState::Running && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }

        if let Some(started) = self.started_at {
            if matches!(state, ManagerState::Running | ManagerState::Paused) {
                self.uptime = Utc::now().signed_duration_since(started).to_std().ok();
            }
        }
    }

    /// Sets the health status
    pub fn set_health(&mut self, health: HealthStatus) {
        self.health = health;
        self.last_updated = Utc::now();
    }

    /// Sets a status message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.last_updated = Utc::now();
    }

    /// Adds metadata to the status
    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
        self.last_updated = Utc::now();
    }

    /// Updates the metrics
    pub fn update_metrics(&mut self, metrics: ManagerMetrics) {
        self.metrics = metrics;
        self.last_updated = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub active_operations: u32,
    pub total_operations: u64,
    pub operations_per_second: f64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
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

// Native platforms require Send + Sync
#[cfg(not(target_arch = "wasm32"))]
pub trait PlatformSync: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> PlatformSync for T {}

// WASM does not require Send
#[cfg(target_arch = "wasm32")]
pub trait PlatformSync {}
#[cfg(target_arch = "wasm32")]
impl<T> PlatformSync for T {}


/// Core trait for all system managers - conditional Send requirement based on target
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait Manager: PlatformSync + fmt::Debug {
    /// Returns the manager name
    fn name(&self) -> &str;

    /// Returns the manager ID
    fn id(&self) -> Uuid;

    /// Initializes the manager
    async fn initialize(&mut self) -> Result<()>;

    /// Shuts down the manager
    async fn shutdown(&mut self) -> Result<()>;

    /// Returns current status
    async fn status(&self) -> ManagerStatus;

    /// Performs health check
    async fn health_check(&self) -> HealthStatus {
        let status = self.status().await;
        match status.state {
            ManagerState::Running => HealthStatus::Healthy,
            ManagerState::Paused | ManagerState::Maintenance => HealthStatus::Degraded,
            ManagerState::Error => HealthStatus::Unhealthy,
            _ => HealthStatus::Unknown,
        }
    }

    /// Pauses the manager
    async fn pause(&mut self) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Pause,
            "Pause operation not supported",
        ))
    }

    /// Resumes the manager
    async fn resume(&mut self) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Resume,
            "Resume operation not supported",
        ))
    }

    /// Restarts the manager
    async fn restart(&mut self) -> Result<()> {
        self.shutdown().await?;
        self.initialize().await
    }

    /// Gets current configuration
    async fn get_config(&self) -> Option<serde_json::Value> {
        None
    }

    /// Updates configuration
    async fn update_config(&mut self, _config: serde_json::Value) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Configure,
            "Configuration update not supported",
        ))
    }

    /// Returns dependencies
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    /// Returns initialization priority
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if manager is essential for system operation
    fn is_essential(&self) -> bool {
        false
    }

    /// Returns manager version
    fn version(&self) -> Option<String> {
        None
    }

    /// Returns manager description
    fn description(&self) -> Option<String> {
        None
    }

    /// Checks if manager supports runtime reloading
    fn supports_runtime_reload(&self) -> bool {
        false
    }

    /// Reloads configuration at runtime
    async fn reload_config(&mut self, _config: serde_json::Value) -> Result<()> {
        Err(Error::manager(
            self.name(),
            ManagerOperation::Reload,
            "Runtime configuration reload not supported",
        ))
    }

    /// Returns required permissions
    fn required_permissions(&self) -> Vec<String> {
        Vec::new()
    }

    /// Returns platform requirements
    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements::default()
    }
}

// #[cfg(target_arch = "wasm32")]
// #[async_trait(?Send)]
// pub trait Manager: Sync + fmt::Debug {
//     /// Returns the manager name
//     fn name(&self) -> &str;
// 
//     /// Returns the manager ID
//     fn id(&self) -> Uuid;
// 
//     /// Initializes the manager
//     async fn initialize(&mut self) -> Result<()>;
// 
//     /// Shuts down the manager
//     async fn shutdown(&mut self) -> Result<()>;
// 
//     /// Returns current status
//     async fn status(&self) -> ManagerStatus;
// 
//     /// Performs health check
//     async fn health_check(&self) -> HealthStatus {
//         let status = self.status().await;
//         match status.state {
//             ManagerState::Running => HealthStatus::Healthy,
//             ManagerState::Paused | ManagerState::Maintenance => HealthStatus::Degraded,
//             ManagerState::Error => HealthStatus::Unhealthy,
//             _ => HealthStatus::Unknown,
//         }
//     }
// 
//     /// Pauses the manager
//     async fn pause(&mut self) -> Result<()> {
//         Err(Error::manager(
//             self.name(),
//             ManagerOperation::Pause,
//             "Pause operation not supported",
//         ))
//     }
// 
//     /// Resumes the manager
//     async fn resume(&mut self) -> Result<()> {
//         Err(Error::manager(
//             self.name(),
//             ManagerOperation::Resume,
//             "Resume operation not supported",
//         ))
//     }
// 
//     /// Restarts the manager
//     async fn restart(&mut self) -> Result<()> {
//         self.shutdown().await?;
//         self.initialize().await
//     }
// 
//     /// Gets current configuration
//     async fn get_config(&self) -> Option<serde_json::Value> {
//         None
//     }
// 
//     /// Updates configuration
//     async fn update_config(&mut self, _config: serde_json::Value) -> Result<()> {
//         Err(Error::manager(
//             self.name(),
//             ManagerOperation::Configure,
//             "Configuration update not supported",
//         ))
//     }
// 
//     /// Returns dependencies
//     fn dependencies(&self) -> Vec<String> {
//         Vec::new()
//     }
// 
//     /// Returns initialization priority
//     fn priority(&self) -> i32 {
//         0
//     }
// 
//     /// Checks if manager is essential for system operation
//     fn is_essential(&self) -> bool {
//         false
//     }
// 
//     /// Returns manager version
//     fn version(&self) -> Option<String> {
//         None
//     }
// 
//     /// Returns manager description
//     fn description(&self) -> Option<String> {
//         None
//     }
// 
//     /// Checks if manager supports runtime reloading
//     fn supports_runtime_reload(&self) -> bool {
//         false
//     }
// 
//     /// Reloads configuration at runtime
//     async fn reload_config(&mut self, _config: serde_json::Value) -> Result<()> {
//         Err(Error::manager(
//             self.name(),
//             ManagerOperation::Reload,
//             "Runtime configuration reload not supported",
//         ))
//     }
// 
//     /// Returns required permissions
//     fn required_permissions(&self) -> Vec<String> {
//         Vec::new()
//     }
// 
//     /// Returns platform requirements
//     fn platform_requirements(&self) -> PlatformRequirements {
//         PlatformRequirements::default()
//     }
// }

/// Managed state container for managers
pub struct ManagedState {
    id: Uuid,
    name: String,
    status: Arc<RwLock<ManagerStatus>>,
}

impl ManagedState {
    /// Creates a new managed state
    pub fn new(id: Uuid, name: impl Into<String>) -> Self {
        let name_str = name.into();
        let status = ManagerStatus::new(id, name_str.clone(), ManagerState::Created);

        Self {
            id,
            name: name_str,
            status: Arc::new(RwLock::new(status)),
        }
    }

    /// Returns the manager ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Returns the manager name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the manager state
    pub async fn set_state(&self, state: ManagerState) {
        let mut status = self.status.write().await;
        status.update_state(state);
    }

    /// Sets the health status
    pub async fn set_health(&self, health: HealthStatus) {
        let mut status = self.status.write().await;
        status.set_health(health);
    }

    /// Sets a status message
    pub async fn set_message(&self, message: impl Into<String>) {
        let mut status = self.status.write().await;
        status.set_message(message);
    }

    /// Adds metadata
    pub async fn add_metadata(&self, key: impl Into<String>, value: serde_json::Value) {
        let mut status = self.status.write().await;
        status.add_metadata(key, value);
    }

    /// Updates metrics
    pub async fn update_metrics(&self, metrics: ManagerMetrics) {
        let mut status = self.status.write().await;
        status.update_metrics(metrics);
    }

    /// Returns current status
    pub async fn status(&self) -> ManagerStatus {
        self.status.read().await.clone()
    }

    /// Returns current state
    pub async fn state(&self) -> ManagerState {
        self.status.read().await.state
    }

    /// Returns current health
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestManager {
        state: ManagedState,
    }

    impl TestManager {
        fn new(name: &str) -> Self {
            Self {
                state: ManagedState::new(Uuid::new_v4(), name),
            }
        }

        async fn current_state(&self) -> ManagerState {
            self.state.state().await
        }
    }

    // #[async_trait]
    // impl Manager for TestManager {
    //     fn name(&self) -> &str {
    //         self.state.name()
    //     }
    // 
    //     fn id(&self) -> Uuid {
    //         self.state.id()
    //     }
    // 
    //     async fn initialize(&mut self) -> Result<()> {
    //         self.state.set_state(ManagerState::Running).await;
    //         Ok(())
    //     }
    // 
    //     async fn shutdown(&mut self) -> Result<()> {
    //         self.state.set_state(ManagerState::Shutdown).await;
    //         Ok(())
    //     }
    // 
    //     async fn status(&self) -> ManagerStatus {
    //         self.state.status().await
    //     }
    // }
    // 
    // #[tokio::test]
    // async fn test_manager_lifecycle() {
    //     let mut manager = TestManager::new("test_manager");
    // 
    //     assert_eq!(manager.name(), "test_manager");
    //     assert_eq!(manager.current_state().await, ManagerState::Created);
    // 
    //     manager.initialize().await.unwrap();
    //     assert_eq!(manager.current_state().await, ManagerState::Running);
    // 
    //     manager.shutdown().await.unwrap();
    //     assert_eq!(manager.current_state().await, ManagerState::Shutdown);
    // 
    // }
    // 
    // #[tokio::test]
    // async fn test_manager_status() {
    //     let manager = TestManager::new("test_manager");
    //     let status = manager.status().await;
    // 
    //     assert_eq!(status.name, "test_manager");
    //     assert_eq!(status.state, ManagerState::Created);
    //     assert_eq!(status.health, HealthStatus::Unknown);
    // }
}