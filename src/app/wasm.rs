// src/app/wasm.rs - Fixed WASM-specific application core with proper plugin integration

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::sync::RwLock;

use crate::auth::{
    AccountManager, MemorySessionStore, MemoryUserStore, SecurityPolicy, User, UserSession,
};
use crate::config::{ConfigurationTier, MemoryConfigStore, TieredConfigManager};
use crate::error::{Result};
use crate::event::EventBusManager;
use crate::manager::{HealthStatus, ManagedState, Manager, ManagerState};
use crate::platform::PlatformManager;
use crate::plugin::{PluginManager, PluginManagerConfig};
use crate::ui::UILayoutManager;
use crate::ui::services::plugin_service::initialize_plugin_service;
use crate::utils::Time;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationState {
    Created,
    Initializing,
    Running,
    ShuttingDown,
    Shutdown,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationHealth {
    pub status: HealthStatus,
    pub uptime: Duration,
    pub managers: HashMap<String, HealthStatus>,
    pub last_check: f64,
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStats {
    pub version: String,
    pub started_at: f64,
    pub uptime: Duration,
    pub state: ApplicationState,
    pub manager_count: usize,
    pub initialized_managers: usize,
    pub failed_managers: usize,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub hostname: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            os_name: "web".to_string(),
            os_version: "1.0".to_string(),
            arch: "wasm32".to_string(),
            cpu_cores: 1, // Simplified for web
            total_memory_bytes: 0,
            available_memory_bytes: 0,
            hostname: "localhost".to_string(),
        }
    }
}

pub struct ApplicationCore {
    state: ManagedState,
    app_state: ApplicationState,
    started_at: f64,

    // Core managers for web
    platform_manager: Option<Arc<PlatformManager>>,
    config_manager: Option<TieredConfigManager>,
    event_bus_manager: Option<Arc<EventBusManager>>,
    account_manager: Option<AccountManager>,
    plugin_manager: Option<Arc<RwLock<PluginManager>>>,
    ui_layout_manager: Option<UILayoutManager>,

    // Current user context
    current_user: Option<User>,
    current_session: Option<UserSession>,

    // System info
    system_info: SystemInfo,
}

impl std::fmt::Debug for ApplicationCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplicationCore")
            .field("started_at", &self.started_at)
            .field("app_state", &self.app_state)
            .field("system_info", &self.system_info)
            .finish()
    }
}

impl ApplicationCore {
    pub fn new() -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "application_core"),
            app_state: ApplicationState::Created,
            started_at: Time::now_millis() as f64,
            platform_manager: None,
            config_manager: None,
            event_bus_manager: None,
            account_manager: None,
            plugin_manager: None,
            ui_layout_manager: None,
            current_user: None,
            current_session: None,
            system_info: SystemInfo::collect(),
        }
    }

    pub fn with_config_file(_config_path: impl AsRef<std::path::Path>) -> Self {
        // Config files not supported in web, use default
        Self::new()
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.app_state = ApplicationState::Initializing;
        self.state.set_state(ManagerState::Initializing).await;

        web_sys::console::log_1(&"Starting Qorzen WASM application initialization".into());

        // 1. Initialize platform manager
        if let Err(e) = self.init_platform_manager().await {
            web_sys::console::error_1(&format!("Platform manager init failed: {}", e).into());
            self.app_state = ApplicationState::Error;
            return Err(e);
        }

        if let Err(e) = self.init_config_manager().await {
            web_sys::console::error_1(&format!("Config manager init failed: {}", e).into());
            self.app_state = ApplicationState::Error;
            return Err(e);
        }

        if let Err(e) = self.init_event_bus_manager().await {
            web_sys::console::error_1(&format!("Event Bus manager init failed: {}", e).into());
            self.app_state = ApplicationState::Error;
            return Err(e);
        }

        if let Err(e) = self.init_account_manager().await {
            web_sys::console::error_1(&format!("Account manager init failed: {}", e).into());
            self.app_state = ApplicationState::Error;
            return Err(e);
        }

        if let Err(e) = self.init_ui_layout_manager().await {
            web_sys::console::error_1(&format!("UI Layout manager init failed: {}", e).into());
            self.app_state = ApplicationState::Error;
            return Err(e);
        }

        if let Err(e) = self.init_plugin_manager().await {
            web_sys::console::error_1(&format!("Plugin manager init failed: {}", e).into());
            self.app_state = ApplicationState::Error;
            return Err(e);
        }

        self.app_state = ApplicationState::Running;
        self.state.set_state(ManagerState::Running).await;

        web_sys::console::log_1(&"Qorzen WASM application initialization complete".into());
        Ok(())
    }

    async fn init_platform_manager(&mut self) -> Result<()> {
        web_sys::console::log_1(&"Initializing platform manager".into());
        let mut platform_manager = PlatformManager::new()?;
        platform_manager.initialize().await?;
        self.platform_manager = Some(Arc::new(platform_manager));
        Ok(())
    }

    async fn init_config_manager(&mut self) -> Result<()> {
        web_sys::console::log_1(&"Initializing configuration manager".into());
        let mut config_manager = TieredConfigManager::new();

        config_manager.add_store(
            ConfigurationTier::System,
            Box::new(MemoryConfigStore::new(ConfigurationTier::System)),
        );
        config_manager.add_store(
            ConfigurationTier::Runtime,
            Box::new(MemoryConfigStore::new(ConfigurationTier::Runtime)),
        );

        config_manager.initialize().await?;
        self.config_manager = Some(config_manager);
        Ok(())
    }

    async fn init_event_bus_manager(&mut self) -> Result<()> {
        web_sys::console::log_1(&"Initializing event bus manager".into());
        let event_config = crate::event::EventBusConfig {
            worker_count: 1, // Limited for web
            queue_capacity: 1000,
            default_timeout: Duration::from_secs(5),
            enable_persistence: false,
            enable_metrics: true,
            batch_size: 50,
            max_retry_delay: Duration::from_secs(10),
        };

        let mut event_bus_manager = EventBusManager::new(event_config);
        event_bus_manager.initialize().await?;
        self.event_bus_manager = Some(Arc::new(event_bus_manager));
        Ok(())
    }

    async fn init_account_manager(&mut self) -> Result<()> {
        web_sys::console::log_1(&"Initializing account manager".into());
        let security_policy = SecurityPolicy::default();
        let session_store = Box::new(MemorySessionStore::new());
        let user_store = Box::new(MemoryUserStore::new());

        let mut account_manager = AccountManager::new(session_store, user_store, security_policy);
        account_manager.initialize().await?;
        self.account_manager = Some(account_manager);
        Ok(())
    }

    async fn init_ui_layout_manager(&mut self) -> Result<()> {
        web_sys::console::log_1(&"Initializing UI layout manager".into());
        let mut ui_layout_manager = UILayoutManager::new();
        ui_layout_manager.initialize().await?;
        self.ui_layout_manager = Some(ui_layout_manager);
        Ok(())
    }

    async fn init_plugin_manager(&mut self) -> Result<()> {
        web_sys::console::log_1(&"Initializing plugin manager".into());

        // Create plugin manager with proper config
        let plugin_config = PluginManagerConfig::default();
        let mut plugin_manager = PluginManager::new(plugin_config);

        // Set up dependencies
        if let Some(ref event_bus) = self.event_bus_manager {
            plugin_manager.set_event_bus(Arc::clone(event_bus));
        }

        if let Some(ref platform_manager) = self.platform_manager {
            plugin_manager.set_platform_manager(Arc::clone(platform_manager));
        }

        // Initialize the plugin manager
        plugin_manager.initialize().await?;

        // Register and load built-in plugins
        plugin_manager.register_builtin_plugins().await?;
        plugin_manager.auto_load_plugins().await?;

        web_sys::console::log_1(&"Plugin manager initialized, registering with service".into());

        // Wrap in Arc for sharing with the plugin service
        let manager_arc = Arc::new(RwLock::new(plugin_manager));

        // Initialize the plugin service with the real plugin manager
        initialize_plugin_service(manager_arc.clone()).await;

        // Store the manager
        self.plugin_manager = Some(manager_arc);

        web_sys::console::log_1(&"Plugin manager and service integration complete".into());
        Ok(())
    }

    // Add method to access plugin manager for UI
    pub fn get_plugin_manager(&self) -> Option<Arc<RwLock<PluginManager>>> {
        self.plugin_manager.clone()
    }

    // Add method to get plugin statistics for UI
    pub async fn get_plugin_stats(&self) -> Option<crate::plugin::PluginStats> {
        if let Some(ref plugin_manager_arc) = self.plugin_manager {
            let plugin_manager = plugin_manager_arc.read().await;
            Some(plugin_manager.get_plugin_stats().await)
        } else {
            None
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.app_state = ApplicationState::ShuttingDown;
        self.state.set_state(ManagerState::ShuttingDown).await;

        web_sys::console::log_1(&"Shutting down Qorzen WASM application".into());

        // Shutdown all managers in reverse order
        if let Some(plugin_manager) = self.plugin_manager.take() {
            if let Ok(mut manager) = Arc::try_unwrap(plugin_manager) {
                let mut manager = manager.into_inner();
                let _ = manager.shutdown().await;
            }
        }

        if let Some(mut ui_layout_manager) = self.ui_layout_manager.take() {
            let _ = ui_layout_manager.shutdown().await;
        }

        if let Some(mut account_manager) = self.account_manager.take() {
            let _ = account_manager.shutdown().await;
        }

        if let Some(event_bus_manager) = self.event_bus_manager.take() {
            if let Ok(mut manager) = Arc::try_unwrap(event_bus_manager) {
                let _ = manager.shutdown().await;
            }
        }

        if let Some(mut config_manager) = self.config_manager.take() {
            let _ = config_manager.shutdown().await;
        }

        if let Some(platform_manager) = self.platform_manager.take() {
            if let Ok(mut manager) = Arc::try_unwrap(platform_manager) {
                let _ = manager.shutdown().await;
            }
        }

        self.app_state = ApplicationState::Shutdown;
        self.state.set_state(ManagerState::Shutdown).await;

        web_sys::console::log_1(&"Qorzen WASM application shutdown complete".into());
        Ok(())
    }

    pub async fn wait_for_shutdown(&self) -> Result<()> {
        // In web, we don't have signal handling like native
        // This could be connected to window.beforeunload or similar
        Ok(())
    }

    pub async fn get_health(&self) -> ApplicationHealth {
        let mut manager_health = HashMap::new();
        let mut overall_healthy = true;

        // Check platform manager
        if let Some(platform_manager) = &self.platform_manager {
            let health = platform_manager.health_check().await;
            if health != HealthStatus::Healthy {
                overall_healthy = false;
            }
            manager_health.insert("platform_manager".to_string(), health);
        }

        // Check plugin manager
        if let Some(plugin_manager_arc) = &self.plugin_manager {
            // For WASM, we'll assume it's healthy if it exists
            manager_health.insert("plugin_manager".to_string(), HealthStatus::Healthy);
        }

        let overall_status = if overall_healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded
        };

        let current_time = Time::now_millis() as f64;
        let uptime = Duration::from_millis((current_time - self.started_at) as u64);

        ApplicationHealth {
            status: overall_status,
            uptime: uptime,
            managers: manager_health,
            last_check: current_time,
            details: HashMap::new(),
        }
    }

    pub async fn get_stats(&self) -> ApplicationStats {
        let current_time = Time::now_millis() as f64;
        let uptime = Duration::from_millis((current_time - self.started_at) as u64);

        ApplicationStats {
            version: crate::VERSION.to_string(),
            started_at: self.started_at,
            uptime: uptime,
            state: self.app_state,
            manager_count: 6, // Approximate count
            initialized_managers: 6,
            failed_managers: 0,
            memory_usage_bytes: 0, // Not available in web
            cpu_usage_percent: 0.0,
            system_info: self.system_info.clone(),
        }
    }

    pub async fn current_user(&self) -> Option<User> {
        self.current_user.clone()
    }

    pub async fn current_session(&self) -> Option<UserSession> {
        self.current_session.clone()
    }

    pub async fn get_state(&self) -> ApplicationState {
        self.app_state
    }
}

impl Default for ApplicationCore {
    fn default() -> Self {
        Self::new()
    }
}

// WASM doesn't need the full Manager trait implementation since it's simpler
// But we can provide a basic one for consistency

#[async_trait::async_trait(?Send)]
impl Manager for ApplicationCore {
    fn name(&self) -> &str {
        "application_core"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        // Main initialization is handled by the public initialize method
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Main shutdown is handled by the public shutdown method
        Ok(())
    }

    async fn status(&self) -> crate::manager::ManagerStatus {
        let mut status = self.state.status().await;
        let app_stats = self.get_stats().await;

        status.add_metadata(
            "app_state",
            serde_json::Value::String(format!("{:?}", app_stats.state)),
        );
        status.add_metadata(
            "uptime_seconds",
            serde_json::Value::from(app_stats.uptime.as_secs()),
        );
        status.add_metadata(
            "manager_count",
            serde_json::Value::from(app_stats.manager_count),
        );
        status.add_metadata("version", serde_json::Value::String(app_stats.version));

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

    #[tokio::test]
    async fn test_wasm_application_lifecycle() {
        let mut app = ApplicationCore::new();

        assert_eq!(app.get_state().await, ApplicationState::Created);

        // Note: Full initialization might fail in test environment
        // due to missing web APIs, but we can test the structure
        assert!(app.get_stats().await.version == crate::VERSION);
    }

    #[test]
    fn test_system_info() {
        let info = SystemInfo::collect();
        assert_eq!(info.os_name, "web");
        assert_eq!(info.arch, "wasm32");
    }
}