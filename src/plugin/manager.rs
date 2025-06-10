use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::{
    config::PluginManagerConfig,
    loader::{PluginInstallationManager, PluginStatus},
    manifest::PluginManifest,
    registry::{builtin, PluginFactoryRegistry},
    search::{SearchCoordinator, SearchProvider},
    ApiRequest, ApiResponse, DatabasePermissions, Plugin, PluginApiClient, PluginConfig,
    PluginContext, PluginDatabase, PluginFileSystem,
};
use crate::error::{Error, Result};
use crate::event::EventBusManager;
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};
use crate::platform::{filesystem::FileSystemArc, PlatformManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationSource {
    Local { path: PathBuf },
    Git { url: String, branch: Option<String> },
    Registry { url: String, plugin_id: String, version: Option<String> },
    Binary { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallRequest {
    pub source: InstallationSource,
    pub force_reinstall: bool,
    pub auto_enable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub download_url: String,
    pub checksum: String,
    pub supported_platforms: Vec<String>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub rating: Option<f32>,
    pub downloads: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct PluginRegistry {
    base_url: String,
    #[cfg(not(target_arch = "wasm32"))]
    client: Option<reqwest::Client>,
}

impl PluginRegistry {
    pub fn new(base_url: String) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let client = Some(reqwest::Client::new());

        Self {
            base_url,
            #[cfg(not(target_arch = "wasm32"))]
            client,
        }
    }

    pub async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<RegistryPlugin>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref client) = self.client {
                let url = format!("{}/search", self.base_url);
                let mut request = client.get(&url).query(&[("q", query)]);

                if let Some(limit) = limit {
                    request = request.query(&[("limit", &limit.to_string())]);
                }

                let response = request.send().await.map_err(|e| {
                    Error::plugin("registry", format!("Failed to search registry: {}", e))
                })?;

                if !response.status().is_success() {
                    return Err(Error::plugin(
                        "registry",
                        format!("Registry search failed: {}", response.status()),
                    ));
                }

                let plugins: Vec<RegistryPlugin> = response.json().await.map_err(|e| {
                    Error::plugin(
                        "registry",
                        format!("Failed to parse registry response: {}", e),
                    )
                })?;

                return Ok(plugins);
            }
        }

        Ok(vec![])
    }

    pub async fn get_plugin(&self, plugin_id: &str) -> Result<Option<RegistryPlugin>> {
        let plugins = self.search(plugin_id, Some(1)).await?;
        Ok(plugins.into_iter().find(|p| p.id == plugin_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub active_plugins: usize,
    pub search_providers: usize,
    pub status_counts: HashMap<PluginStatus, usize>,
}

fn get_plugin_data_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("QORZEN_PLUGINS_DIR") {
        return PathBuf::from(env_dir);
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(".local/share/qorzen/plugins");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            return data_dir.join("qorzen/plugins");
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            return data_dir.join("Qorzen/plugins");
        }
    }

    PathBuf::from("./target/plugins")
}

#[derive(Debug)]
pub struct PluginManager {
    state: ManagedState,
    config: PluginManagerConfig,
    installation_manager: Arc<Mutex<PluginInstallationManager>>,
    search_coordinator: Arc<SearchCoordinator>,
    registry: Arc<PluginRegistry>,
    event_bus: Option<Arc<EventBusManager>>,
    platform_manager: Option<Arc<PlatformManager>>,
    active_plugins: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn Plugin>>>>>>,
    plugin_contexts: Arc<RwLock<HashMap<String, PluginContext>>>,
    search_providers: Arc<RwLock<HashMap<String, Arc<dyn SearchProvider>>>>,
    plugin_registry: Arc<RwLock<HashMap<String, PluginManifest>>>,
    plugins_directory: PathBuf,
}

impl PluginManager {
    pub fn new(config: PluginManagerConfig) -> Self {
        let plugins_directory = get_plugin_data_dir();

        let installation_manager = Arc::new(Mutex::new(PluginInstallationManager::new(
            plugins_directory.clone(),
        )));

        Self {
            state: ManagedState::new(Uuid::new_v4(), "plugin_manager"),
            config,
            installation_manager,
            search_coordinator: Arc::new(SearchCoordinator::new()),
            registry: Arc::new(PluginRegistry::new(
                "https://registry.qorzen.com".to_string(),
            )),
            event_bus: None,
            platform_manager: None,
            active_plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_contexts: Arc::new(RwLock::new(HashMap::new())),
            search_providers: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry: Arc::new(RwLock::new(HashMap::new())),
            plugins_directory,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(PluginManagerConfig::default())
    }

    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    pub fn set_platform_manager(&mut self, platform_manager: Arc<PlatformManager>) {
        self.platform_manager = Some(platform_manager);
    }

    pub async fn load_config(&mut self, config_path: Option<&str>) -> Result<()> {
        let config_file = config_path.unwrap_or("plugins.toml");

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(content) = tokio::fs::read_to_string(config_file).await {
                match toml::from_str::<PluginManagerConfig>(&content) {
                    Ok(config) => {
                        self.config = config;
                        tracing::info!("Loaded plugin configuration from {}", config_file);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse plugin configuration ({}): {}, using defaults",
                            config_file,
                            e
                        );
                    }
                }
            } else {
                tracing::info!("No plugin configuration file found, using defaults");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            tracing::info!("Using default plugin configuration for WASM environment");
        }

        Ok(())
    }

    pub async fn register_builtin_plugins(&self) -> Result<()> {
        builtin::register_builtin_plugins().await?;
        tracing::info!("Registered built-in plugins");
        Ok(())
    }

    pub async fn search_registry(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<RegistryPlugin>> {
        self.registry.search(query, limit).await
    }

    pub async fn load_plugin(&self, plugin_id: &str) -> Result<()> {
        tracing::info!("Loading plugin: {}", plugin_id);

        if self.active_plugins.read().await.contains_key(plugin_id) {
            return Err(Error::plugin(plugin_id, "Plugin already loaded"));
        }

        if let Some(mut plugin) = PluginFactoryRegistry::create_plugin(plugin_id).await {
            let context = self.create_plugin_context_for_builtin(plugin_id).await?;
            plugin.initialize(context.clone()).await?;

            self.active_plugins
                .write()
                .await
                .insert(plugin_id.to_string(), Arc::new(Mutex::new(plugin)));
            self.plugin_contexts
                .write()
                .await
                .insert(plugin_id.to_string(), context);

            tracing::info!("Plugin {} loaded from factory registry", plugin_id);
            return Ok(());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let installation_manager = self.installation_manager.lock().await;

            if let Err(e) = installation_manager.discover_plugins().await {
                tracing::warn!("Failed to discover plugins: {}", e);
            }

            if let Some(installation) = installation_manager.get_installation(plugin_id).await {
                let context = self.create_plugin_context(&installation.manifest).await?;

                self.plugin_contexts
                    .write()
                    .await
                    .insert(plugin_id.to_string(), context.clone());

                let plugin = installation_manager
                    .load_plugin(plugin_id, context)
                    .await?;

                self.active_plugins
                    .write()
                    .await
                    .insert(plugin_id.to_string(), Arc::new(Mutex::new(plugin)));

                self.plugin_registry
                    .write()
                    .await
                    .insert(plugin_id.to_string(), installation.manifest.clone());

                installation_manager
                    .update_status(plugin_id, PluginStatus::Running)
                    .await;

                tracing::info!("Plugin {} loaded from installation", plugin_id);
                return Ok(());
            }
        }

        Err(Error::plugin(
            plugin_id,
            "Plugin not found in factory registry or installations",
        ))
    }

    pub async fn auto_load_plugins(&self) -> Result<()> {
        if !self.config.auto_load {
            tracing::info!("Plugin auto-loading is disabled");
            return Ok(());
        }

        tracing::info!("Auto-loading plugins...");

        for plugin_id in &self.config.default_plugins {
            if !self.active_plugins.read().await.contains_key(plugin_id) {
                tracing::info!("Auto-loading configured plugin: {}", plugin_id);
                if let Err(e) = self.load_plugin(plugin_id).await {
                    tracing::error!("Failed to auto-load configured plugin {}: {}", plugin_id, e);
                }
            } else {
                tracing::debug!("Plugin {} already loaded, skipping", plugin_id);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let installation_manager = self.installation_manager.lock().await;
            let discovered = installation_manager.discover_plugins().await?;
            drop(installation_manager);

            for plugin_id in discovered {
                if !self.active_plugins.read().await.contains_key(&plugin_id) {
                    if self.should_auto_load_plugin(&plugin_id).await {
                        tracing::info!("Auto-loading discovered plugin: {}", plugin_id);
                        if let Err(e) = self.load_plugin(&plugin_id).await {
                            tracing::error!(
                                "Failed to auto-load discovered plugin {}: {}",
                                plugin_id,
                                e
                            );
                        }
                    }
                } else {
                    tracing::debug!("Discovered plugin {} already loaded, skipping", plugin_id);
                }
            }
        }

        let loaded_count = self.active_plugins.read().await.len();
        tracing::info!("Auto-loading complete. {} plugins loaded", loaded_count);

        Ok(())
    }

    async fn should_auto_load_plugin(&self, plugin_id: &str) -> bool {
        if self.config.default_plugins.contains(&plugin_id.to_string()) {
            return true;
        }

        if let Ok(installation_manager) = self.installation_manager.try_lock() {
            if let Some(installation) = installation_manager.get_installation(plugin_id).await {
                if let Some(auto_load) = installation
                    .manifest
                    .settings
                    .as_ref()
                    .and_then(|s| s.get("auto_load"))
                    .and_then(|v| v.as_bool())
                {
                    return auto_load;
                }
            }
        }

        false
    }

    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<()> {
        tracing::info!("Stopping plugin: {}", plugin_id);

        if let Some(plugin_arc) = self.active_plugins.write().await.remove(plugin_id) {
            let mut plugin = plugin_arc.lock().await;
            plugin.shutdown().await?;

            let installation_manager = self.installation_manager.lock().await;
            installation_manager
                .update_status(plugin_id, PluginStatus::Stopped)
                .await;

            tracing::info!("Plugin {} stopped successfully", plugin_id);
        } else {
            tracing::warn!("Plugin {} was not loaded", plugin_id);
        }

        Ok(())
    }

    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        tracing::info!("Uninstalling plugin: {}", plugin_id);

        if self.active_plugins.read().await.contains_key(plugin_id) {
            self.stop_plugin(plugin_id).await?;
        }

        let installation_manager = self.installation_manager.lock().await;
        installation_manager.uninstall_plugin(plugin_id).await?;

        self.plugin_contexts.write().await.remove(plugin_id);
        self.plugin_registry.write().await.remove(plugin_id);

        tracing::info!("Plugin {} uninstalled successfully", plugin_id);
        Ok(())
    }

    pub async fn render_component(
        &self,
        plugin_id: &str,
        component_id: &str,
        props: serde_json::Value,
    ) -> Result<dioxus::prelude::VNode> {
        let plugins = self.active_plugins.read().await;
        if let Some(plugin_arc) = plugins.get(plugin_id) {
            let plugin = plugin_arc.lock().await;
            plugin.render_component(component_id, props)
        } else {
            Err(Error::plugin(plugin_id, "Plugin not loaded"))
        }
    }

    pub async fn handle_api_request(
        &self,
        plugin_id: &str,
        route_id: &str,
        request: ApiRequest,
    ) -> Result<ApiResponse> {
        let plugins = self.active_plugins.read().await;
        if let Some(plugin_arc) = plugins.get(plugin_id) {
            let plugin = plugin_arc.lock().await;
            plugin.handle_api_request(route_id, request).await
        } else {
            Err(Error::plugin(plugin_id, "Plugin not loaded"))
        }
    }

    pub async fn get_plugin_stats(&self) -> PluginStats {
        let installation_manager = self.installation_manager.lock().await;
        let installations = installation_manager.list_installations().await;
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

    pub async fn get_all_ui_components(&self) -> Vec<(String, super::UIComponent)> {
        let mut components = Vec::new();
        let plugins = self.active_plugins.read().await;

        for (plugin_id, plugin_arc) in plugins.iter() {
            let plugin = plugin_arc.lock().await;
            for component in plugin.ui_components() {
                components.push((plugin_id.clone(), component));
            }
        }

        components
    }

    pub async fn get_all_menu_items(&self) -> Vec<(String, super::MenuItem)> {
        let mut items = Vec::new();
        let plugins = self.active_plugins.read().await;

        for (plugin_id, plugin_arc) in plugins.iter() {
            let plugin = plugin_arc.lock().await;
            for item in plugin.menu_items() {
                items.push((plugin_id.clone(), item));
            }
        }

        items
    }

    pub async fn get_loaded_plugins(&self) -> Vec<super::PluginInfo> {
        let mut plugin_infos = Vec::new();
        let plugins = self.active_plugins.read().await;

        for plugin_arc in plugins.values() {
            let plugin = plugin_arc.lock().await;
            plugin_infos.push(plugin.info());
        }

        plugin_infos
    }

    pub async fn is_plugin_loaded(&self, plugin_id: &str) -> bool {
        self.active_plugins.read().await.contains_key(plugin_id)
    }

    pub async fn refresh_plugins(&self) -> Result<Vec<String>> {
        let installation_manager = self.installation_manager.lock().await;
        installation_manager.discover_plugins().await
    }

    async fn create_plugin_context_for_builtin(&self, plugin_id: &str) -> Result<PluginContext> {
        let api_client = PluginApiClient::new(plugin_id.to_string());
        let event_bus = self
            .event_bus
            .as_ref()
            .ok_or_else(|| Error::plugin(plugin_id, "Event bus not available"))?
            .clone();

        let file_system = if let Some(platform_manager) = &self.platform_manager {
            let fs_provider = platform_manager.filesystem_arc();
            PluginFileSystem::new(plugin_id.to_string(), fs_provider)
        } else {
            let mock_fs: FileSystemArc = Arc::new(crate::platform::MockFileSystem::new());
            PluginFileSystem::new(plugin_id.to_string(), mock_fs)
        };

        let config = PluginConfig {
            plugin_id: plugin_id.to_string(),
            version: "1.0.0".to_string(),
            config_schema: self
                .config
                .plugin_configs
                .get(plugin_id)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            default_values: serde_json::json!({}),
            user_overrides: self
                .config
                .plugin_configs
                .get(plugin_id)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            validation_rules: vec![],
        };

        Ok(PluginContext {
            plugin_id: plugin_id.to_string(),
            config,
            api_client,
            event_bus,
            database: None,
            file_system,
        })
    }

    async fn create_plugin_context(&self, manifest: &PluginManifest) -> Result<PluginContext> {
        let plugin_id = manifest.plugin.id.clone();
        let api_client = PluginApiClient::new(plugin_id.clone());
        let event_bus = self
            .event_bus
            .as_ref()
            .ok_or_else(|| Error::plugin(&plugin_id, "Event bus not available"))?
            .clone();

        let file_system = if let Some(platform_manager) = &self.platform_manager {
            let fs_provider = platform_manager.filesystem_arc();
            PluginFileSystem::new(plugin_id.clone(), fs_provider)
        } else {
            let mock_fs: FileSystemArc = Arc::new(crate::platform::MockFileSystem::new());
            PluginFileSystem::new(plugin_id.clone(), mock_fs)
        };

        let database = if manifest.requires.contains(&"database.query".to_string()) {
            if let Some(platform_manager) = &self.platform_manager {
                let db_provider = platform_manager.database_arc();
                Some(PluginDatabase::new(
                    plugin_id.clone(),
                    db_provider,
                    DatabasePermissions {
                        can_create_tables: false,
                        can_drop_tables: false,
                        can_modify_schema: false,
                        max_table_count: Some(10),
                        max_storage_size: Some(100 * 1024 * 1024),
                    },
                ))
            } else {
                None
            }
        } else {
            None
        };

        let config = PluginConfig {
            plugin_id: plugin_id.clone(),
            version: manifest.plugin.version.clone(),
            config_schema: manifest
                .settings
                .clone()
                .unwrap_or(serde_json::json!({})),
            default_values: manifest
                .settings
                .clone()
                .unwrap_or(serde_json::json!({})),
            user_overrides: self
                .config
                .plugin_configs
                .get(&plugin_id)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
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
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Manager for PluginManager {
    fn name(&self) -> &str {
        "plugin_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        tracing::info!("Initializing plugin manager...");
        tracing::info!("Plugin directory: {}", self.plugins_directory.display());

        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.plugins_directory.exists() {
                if let Err(e) = tokio::fs::create_dir_all(&self.plugins_directory).await {
                    tracing::warn!(
                        "Failed to create plugins directory {}: {}",
                        self.plugins_directory.display(),
                        e
                    );
                } else {
                    tracing::info!(
                        "Created plugins directory: {}",
                        self.plugins_directory.display()
                    );
                }
            }
        }

        let mut installation_manager = self.installation_manager.lock().await;
        if let Err(e) = installation_manager.initialize().await {
            tracing::error!("Failed to initialize installation manager: {}", e);
        }
        drop(installation_manager);

        if let Err(e) = self.register_builtin_plugins().await {
            tracing::warn!("Failed to register built-in plugins: {}", e);
        }

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;

        tracing::info!("Plugin manager initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        tracing::info!("Shutting down plugin manager...");

        let plugin_ids: Vec<String> = self.active_plugins.read().await.keys().cloned().collect();
        for plugin_id in plugin_ids {
            if let Err(e) = self.stop_plugin(&plugin_id).await {
                tracing::error!("Failed to stop plugin {}: {}", plugin_id, e);
            }
        }

        self.installation_manager.lock().await.shutdown().await?;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;

        tracing::info!("Plugin manager shut down");
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_plugin_stats().await;

        status.add_metadata("total_plugins", serde_json::Value::from(stats.total_plugins));
        status.add_metadata("active_plugins", serde_json::Value::from(stats.active_plugins));
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
            serde_json::Value::Bool(self.config.auto_load),
        );
        status.add_metadata(
            "hot_reload_enabled",
            serde_json::Value::Bool(self.config.hot_reload),
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
}