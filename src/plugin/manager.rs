// src/plugin/manager.rs - Enhanced plugin manager

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::{
    loader::{PluginInstallationManager, PluginStatus},
    manifest::PluginManifest,
    search::{SearchCoordinator, SearchProvider},
    Plugin, PluginApiClient, PluginContext, PluginFileSystem,
};
use crate::config::SettingsSchema;
use crate::error::{Error, Result};
use crate::event::{Event, EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};
use crate::platform::{filesystem::FileSystemArc, PlatformManager};

/// Enhanced plugin manager that orchestrates the entire plugin system
#[derive(Debug)]
pub struct PluginManager {
    state: ManagedState,

    // Core components
    installation_manager: Arc<Mutex<PluginInstallationManager>>,
    #[allow(dead_code)]
    search_coordinator: Arc<SearchCoordinator>,
    event_bus: Option<Arc<EventBusManager>>,
    platform_manager: Option<Arc<PlatformManager>>,

    // Active plugins
    active_plugins: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn Plugin>>>>>>,
    plugin_contexts: Arc<RwLock<HashMap<String, PluginContext>>>,

    // Search providers from plugins
    search_providers: Arc<RwLock<HashMap<String, Arc<dyn SearchProvider>>>>,

    // Plugin registry for metadata
    plugin_registry: Arc<RwLock<HashMap<String, PluginManifest>>>,

    // Configuration
    plugins_directory: PathBuf,
    auto_load_plugins: bool,
    hot_reload_enabled: bool,
}

#[allow(dead_code)]
impl PluginManager {
    /// Create a new enhanced plugin manager
    pub fn new(plugins_directory: PathBuf) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "enhanced_plugin_manager"),
            installation_manager: Arc::new(Mutex::new(PluginInstallationManager::new(
                plugins_directory.clone(),
            ))),
            search_coordinator: Arc::new(SearchCoordinator::new()),
            event_bus: None,
            platform_manager: None,
            active_plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_contexts: Arc::new(RwLock::new(HashMap::new())),
            search_providers: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry: Arc::new(RwLock::new(HashMap::new())),
            plugins_directory,
            auto_load_plugins: true,
            hot_reload_enabled: cfg!(debug_assertions),
        }
    }

    /// Set event bus for plugin communication
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    /// Set platform manager for platform-specific operations
    pub fn set_platform_manager(&mut self, platform_manager: Arc<PlatformManager>) {
        self.platform_manager = Some(platform_manager);
    }

    /// Enable or disable auto-loading of plugins
    pub fn set_auto_load(&mut self, auto_load: bool) {
        self.auto_load_plugins = auto_load;
    }

    /// Enable or disable hot reloading
    pub fn set_hot_reload(&mut self, enabled: bool) {
        self.hot_reload_enabled = enabled;
    }

    /// Install a plugin from a package or directory
    pub async fn install_plugin(&self, source: &str, force: bool) -> Result<String> {
        let installation_manager = self.installation_manager.lock().await;
        installation_manager.install_plugin(source, force).await
    }

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        // Stop the plugin first if it's running
        self.stop_plugin(plugin_id).await?;

        // Remove from registries
        self.active_plugins.write().await.remove(plugin_id);
        self.plugin_contexts.write().await.remove(plugin_id);
        self.plugin_registry.write().await.remove(plugin_id);

        // Unregister search provider if exists
        if self
            .search_providers
            .write()
            .await
            .remove(plugin_id)
            .is_some()
        {
            self.search_coordinator
                .unregister_provider(plugin_id)
                .await?;
        }

        // Uninstall from installation manager
        let installation_manager = self.installation_manager.lock().await;
        installation_manager.uninstall_plugin(plugin_id).await
    }

    /// Load and start a plugin
    pub async fn load_plugin(&self, plugin_id: &str) -> Result<()> {
        // Check if already loaded
        if self.active_plugins.read().await.contains_key(plugin_id) {
            return Ok(());
        }

        // Get installation info
        let installation_manager = self.installation_manager.lock().await;
        let installation = installation_manager
            .get_installation(plugin_id)
            .await
            .ok_or_else(|| Error::plugin(plugin_id, "Plugin not found"))?;

        // Create plugin context
        let context = self.create_plugin_context(&installation.manifest).await?;
        self.plugin_contexts
            .write()
            .await
            .insert(plugin_id.to_string(), context.clone());

        // Load the plugin
        let plugin = installation_manager.load_plugin(plugin_id, context).await?;

        // Store the plugin
        self.active_plugins
            .write()
            .await
            .insert(plugin_id.to_string(), Arc::new(Mutex::new(plugin)));

        // Store manifest
        self.plugin_registry
            .write()
            .await
            .insert(plugin_id.to_string(), installation.manifest.clone());

        // Register search providers if plugin provides them
        if let Some(search_config) = &installation.manifest.search {
            for provider_config in &search_config.providers {
                // For this example, we'll create a simple provider wrapper
                // In practice, plugins would register their own providers
                tracing::info!(
                    "Plugin {} provides search provider: {}",
                    plugin_id,
                    provider_config.id
                );
            }
        }

        // Update installation status
        installation_manager
            .update_status(plugin_id, PluginStatus::Running)
            .await;

        tracing::info!("Plugin {} loaded and started successfully", plugin_id);
        Ok(())
    }

    /// Stop a plugin
    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<()> {
        if let Some(plugin_arc) = self.active_plugins.write().await.remove(plugin_id) {
            let mut plugin = plugin_arc.lock().await;
            plugin.shutdown().await?;

            // Update status
            let installation_manager = self.installation_manager.lock().await;
            installation_manager
                .update_status(plugin_id, PluginStatus::Stopped)
                .await;

            tracing::info!("Plugin {} stopped successfully", plugin_id);
        }
        Ok(())
    }

    /// Restart a plugin
    pub async fn restart_plugin(&self, plugin_id: &str) -> Result<()> {
        self.stop_plugin(plugin_id).await?;
        self.load_plugin(plugin_id).await
    }

    /// Hot reload a plugin (if supported)
    pub async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<()> {
        if !self.hot_reload_enabled {
            return Err(Error::plugin(plugin_id, "Hot reload is disabled"));
        }

        // Check if plugin supports hot reload
        let manifest = self
            .plugin_registry
            .read()
            .await
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| Error::plugin(plugin_id, "Plugin not found"))?;

        if !manifest.build.hot_reload {
            return Err(Error::plugin(
                plugin_id,
                "Plugin does not support hot reload",
            ));
        }

        // Perform hot reload
        self.restart_plugin(plugin_id).await
    }

    /// Get all active plugins
    pub async fn list_active_plugins(&self) -> Vec<String> {
        self.active_plugins.read().await.keys().cloned().collect()
    }

    /// Get plugin information
    pub async fn get_plugin_info(&self, plugin_id: &str) -> Option<PluginManifest> {
        self.plugin_registry.read().await.get(plugin_id).cloned()
    }

    /// Get all UI components from active plugins
    pub async fn get_all_ui_components(&self) -> Vec<(String, super::UIComponent)> {
        let mut components = Vec::new();
        let active_plugins = self.active_plugins.read().await;

        for (plugin_id, plugin_arc) in active_plugins.iter() {
            let plugin = plugin_arc.lock().await;
            for component in plugin.ui_components() {
                components.push((plugin_id.clone(), component));
            }
        }

        components
    }

    /// Get all menu items from active plugins
    pub async fn get_all_menu_items(&self) -> Vec<(String, super::MenuItem)> {
        let mut items = Vec::new();
        let active_plugins = self.active_plugins.read().await;

        for (plugin_id, plugin_arc) in active_plugins.iter() {
            let plugin = plugin_arc.lock().await;
            for item in plugin.menu_items() {
                items.push((plugin_id.clone(), item));
            }
        }

        // Sort by order
        items.sort_by_key(|(_, item)| item.order);
        items
    }

    /// Get all API routes from active plugins
    pub async fn get_all_api_routes(&self) -> Vec<(String, super::ApiRoute)> {
        let mut routes = Vec::new();
        let active_plugins = self.active_plugins.read().await;

        for (plugin_id, plugin_arc) in active_plugins.iter() {
            let plugin = plugin_arc.lock().await;
            for route in plugin.api_routes() {
                routes.push((plugin_id.clone(), route));
            }
        }

        routes
    }

    /// Render a plugin component
    pub async fn render_component(
        &self,
        plugin_id: &str,
        component_id: &str,
        props: serde_json::Value,
    ) -> Result<dioxus::prelude::VNode> {
        let active_plugins = self.active_plugins.read().await;
        if let Some(plugin_arc) = active_plugins.get(plugin_id) {
            let plugin = plugin_arc.lock().await;
            plugin.render_component(component_id, props)
        } else {
            Err(Error::plugin(plugin_id, "Plugin not active"))
        }
    }

    /// Handle API request for a plugin
    pub async fn handle_api_request(
        &self,
        plugin_id: &str,
        route_id: &str,
        request: super::ApiRequest,
    ) -> Result<super::ApiResponse> {
        let active_plugins = self.active_plugins.read().await;
        if let Some(plugin_arc) = active_plugins.get(plugin_id) {
            let plugin = plugin_arc.lock().await;
            plugin.handle_api_request(route_id, request).await
        } else {
            Err(Error::plugin(plugin_id, "Plugin not active"))
        }
    }

    /// Get search coordinator for external access
    pub fn search_coordinator(&self) -> Arc<SearchCoordinator> {
        Arc::clone(&self.search_coordinator)
    }

    /// Update plugin settings
    pub async fn update_plugin_settings(
        &self,
        plugin_id: &str,
        settings: serde_json::Value,
    ) -> Result<()> {
        let installation_manager = self.installation_manager.lock().await;
        installation_manager
            .update_settings(plugin_id, settings)
            .await?;

        // If plugin is active, we might need to restart it to apply new settings
        if self.active_plugins.read().await.contains_key(plugin_id) {
            tracing::info!(
                "Settings updated for plugin {}, consider restarting",
                plugin_id
            );
        }

        Ok(())
    }

    /// Get plugin settings schema
    pub async fn get_plugin_settings_schema(&self, plugin_id: &str) -> Option<SettingsSchema> {
        let active_plugins = self.active_plugins.read().await;
        if let Some(plugin_arc) = active_plugins.get(plugin_id) {
            let plugin = plugin_arc.lock().await;
            plugin.settings_schema()
        } else {
            None
        }
    }

    /// Discover and auto-load plugins
    async fn discover_and_load_plugins(&self) -> Result<()> {
        let installation_manager = self.installation_manager.lock().await;
        let discovered = installation_manager.discover_plugins().await?;

        if self.auto_load_plugins {
            drop(installation_manager); // Release lock before async operations

            for plugin_id in discovered {
                if let Err(e) = self.load_plugin(&plugin_id).await {
                    tracing::error!("Failed to auto-load plugin {}: {}", plugin_id, e);
                }
            }
        }

        Ok(())
    }

    /// Create plugin context for a plugin
    async fn create_plugin_context(&self, manifest: &PluginManifest) -> Result<PluginContext> {
        let plugin_id = manifest.plugin.id.clone();

        // Create API client
        let api_client = PluginApiClient::new(plugin_id.clone());

        // Get event bus reference
        let event_bus = self
            .event_bus
            .as_ref()
            .ok_or_else(|| Error::plugin(&plugin_id, "Event bus not available"))?
            .clone();

        // Create file system access
        let file_system = if let Some(platform_manager) = &self.platform_manager {
            let fs_provider = platform_manager.filesystem_arc();
            PluginFileSystem::new(plugin_id.clone(), fs_provider)
        } else {
            // Create a mock filesystem for testing
            let mock_fs: FileSystemArc = Arc::new(crate::platform::MockFileSystem::new());
            PluginFileSystem::new(plugin_id.clone(), mock_fs)
        };

        // Create database access if required
        let database = if manifest.requires.contains(&"database.query".to_string()) {
            if let Some(platform_manager) = &self.platform_manager {
                let db_provider = platform_manager.database_arc();
                Some(super::PluginDatabase::new(
                    plugin_id.clone(),
                    db_provider,
                    super::DatabasePermissions {
                        can_create_tables: false,
                        can_drop_tables: false,
                        can_modify_schema: false,
                        max_table_count: Some(10),
                        max_storage_size: Some(100 * 1024 * 1024), // 100MB
                    },
                ))
            } else {
                None
            }
        } else {
            None
        };

        // Create plugin configuration from manifest settings
        let config = super::PluginConfig {
            plugin_id: plugin_id.clone(),
            version: manifest.plugin.version.clone(),
            config_schema: manifest.settings.clone().unwrap_or(serde_json::json!({})),
            default_values: manifest.settings.clone().unwrap_or(serde_json::json!({})),
            user_overrides: serde_json::json!({}),
            validation_rules: vec![],
        };

        Ok(PluginContext {
            plugin_id,
            config,
            api_client,
            event_bus,
            database,
            file_system,
        })
    }

    /// Handle plugin events
    pub async fn handle_plugin_event(&self, event: &dyn Event) -> Result<()> {
        let active_plugins = self.active_plugins.read().await;

        for (plugin_id, plugin_arc) in active_plugins.iter() {
            let plugin = plugin_arc.lock().await;
            let event_handlers = plugin.event_handlers();

            for handler in event_handlers {
                if handler.event_type == event.event_type() || handler.event_type == "*" {
                    if let Err(e) = plugin.handle_event(&handler.handler_id, event).await {
                        tracing::error!(
                            "Plugin {} failed to handle event {}: {}",
                            plugin_id,
                            event.event_type(),
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Get plugin statistics
    pub async fn get_plugin_stats(&self) -> PluginStats {
        let installations = {
            let installation_manager = self.installation_manager.lock().await;
            installation_manager.list_installations().await
        };

        let active_count = self.active_plugins.read().await.len();
        let search_providers_count = self.search_providers.read().await.len();

        let mut status_counts = HashMap::new();
        for installation in &installations {
            *status_counts.entry(installation.status).or_insert(0) += 1;
        }

        PluginStats {
            total_plugins: installations.len(),
            active_plugins: active_count,
            search_providers: search_providers_count,
            status_counts,
        }
    }
}

/// Plugin statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub active_plugins: usize,
    pub search_providers: usize,
    pub status_counts: HashMap<PluginStatus, usize>,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Manager for PluginManager {
    fn name(&self) -> &str {
        "enhanced_plugin_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Initialize installation manager
        self.installation_manager.lock().await.initialize().await?;

        // Discover and load plugins
        self.discover_and_load_plugins().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;

        tracing::info!("Enhanced plugin manager initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Stop all active plugins
        let plugin_ids: Vec<String> = self.active_plugins.read().await.keys().cloned().collect();
        for plugin_id in plugin_ids {
            if let Err(e) = self.stop_plugin(&plugin_id).await {
                tracing::error!("Failed to stop plugin {}: {}", plugin_id, e);
            }
        }

        // Shutdown installation manager
        self.installation_manager.lock().await.shutdown().await?;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;

        tracing::info!("Enhanced plugin manager shut down");
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_plugin_stats().await;

        status.add_metadata(
            "total_plugins",
            serde_json::Value::from(stats.total_plugins),
        );
        status.add_metadata(
            "active_plugins",
            serde_json::Value::from(stats.active_plugins),
        );
        status.add_metadata(
            "search_providers",
            serde_json::Value::from(stats.search_providers),
        );
        status.add_metadata(
            "plugins_directory",
            serde_json::Value::String(self.plugins_directory.display().to_string()),
        );
        status.add_metadata(
            "auto_load_enabled",
            serde_json::Value::Bool(self.auto_load_plugins),
        );
        status.add_metadata(
            "hot_reload_enabled",
            serde_json::Value::Bool(self.hot_reload_enabled),
        );

        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec![
                "plugin.load".to_string(),
                "plugin.manage".to_string(),
                "filesystem.read".to_string(),
                "filesystem.write".to_string(),
            ],
        }
    }

    fn supports_runtime_reload(&self) -> bool {
        true
    }

    async fn reload_config(&mut self, config: serde_json::Value) -> Result<()> {
        if let Ok(auto_load) = serde_json::from_value::<bool>(
            config
                .get("auto_load_plugins")
                .cloned()
                .unwrap_or(serde_json::Value::Bool(true)),
        ) {
            self.auto_load_plugins = auto_load;
        }

        if let Ok(hot_reload) = serde_json::from_value::<bool>(
            config
                .get("hot_reload_enabled")
                .cloned()
                .unwrap_or(serde_json::Value::Bool(cfg!(debug_assertions))),
        ) {
            self.hot_reload_enabled = hot_reload;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_enhanced_plugin_manager_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().to_path_buf();

        let mut manager = PluginManager::new(plugins_dir);
        manager.initialize().await.unwrap();

        let status = manager.status().await;
        assert_eq!(status.state, crate::manager::ManagerState::Running);

        let stats = manager.get_plugin_stats().await;
        assert_eq!(stats.active_plugins, 0); // No plugins in temp directory

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_plugin_stats() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().to_path_buf();

        let manager = PluginManager::new(plugins_dir);
        let stats = manager.get_plugin_stats().await;

        assert_eq!(stats.total_plugins, 0);
        assert_eq!(stats.active_plugins, 0);
    }

    #[tokio::test]
    async fn test_plugin_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().to_path_buf();

        let mut manager = PluginManager::new(plugins_dir);

        // Test configuration changes
        let config = serde_json::json!({
            "auto_load_plugins": false,
            "hot_reload_enabled": true
        });

        manager.reload_config(config).await.unwrap();
        assert!(!manager.auto_load_plugins);
        assert!(manager.hot_reload_enabled);
    }
}
