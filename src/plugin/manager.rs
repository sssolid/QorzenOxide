// src/plugin/manager.rs - Primary plugin manager with full lifecycle management

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

/// Plugin installation source types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationSource {
    /// Local directory containing plugin
    Local { path: PathBuf },
    /// Git repository URL
    Git { url: String, branch: Option<String> },
    /// Plugin registry URL
    Registry {
        url: String,
        plugin_id: String,
        version: Option<String>,
    },
    /// Pre-compiled binary
    Binary { path: PathBuf },
}

/// Plugin installation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallRequest {
    pub source: InstallationSource,
    pub force_reinstall: bool,
    pub auto_enable: bool,
}

/// Plugin registry entry for remote plugins
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

/// Plugin registry client for remote plugin management
#[derive(Debug)]
pub struct PluginRegistry {
    base_url: String,
    #[cfg(not(target_arch = "wasm32"))]
    client: Option<reqwest::Client>,
}

impl PluginRegistry {
    /// Create a new plugin registry client
    pub fn new(base_url: String) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let client = Some(reqwest::Client::new());

        Self {
            base_url,
            #[cfg(not(target_arch = "wasm32"))]
            client,
        }
    }

    /// Search for plugins in the registry
    #[allow(unused_variables)]
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

        // WASM fallback or no client available - return mock data
        Ok(vec![RegistryPlugin {
            id: "example_plugin".to_string(),
            name: "Example Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "An example plugin for demonstration".to_string(),
            author: "Qorzen Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://example.com".to_string()),
            repository: Some("https://github.com/qorzen/example-plugin".to_string()),
            download_url: "https://registry.qorzen.com/plugins/example_plugin/1.0.0/download"
                .to_string(),
            checksum: "sha256:1234567890abcdef".to_string(),
            supported_platforms: vec!["web".to_string(), "desktop".to_string()],
            dependencies: vec![],
            tags: vec!["example".to_string(), "demo".to_string()],
            rating: Some(4.5),
            downloads: 1250,
            last_updated: chrono::Utc::now(),
        }])
    }

    /// Get plugin details by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<Option<RegistryPlugin>> {
        let plugins = self.search(plugin_id, Some(1)).await?;
        Ok(plugins.into_iter().find(|p| p.id == plugin_id))
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

/// Main enhanced plugin manager
#[derive(Debug)]
pub struct PluginManager {
    state: ManagedState,
    config: PluginManagerConfig,

    // Core components
    installation_manager: Arc<Mutex<PluginInstallationManager>>,
    search_coordinator: Arc<SearchCoordinator>,
    registry: Arc<PluginRegistry>,

    // External dependencies
    event_bus: Option<Arc<EventBusManager>>,
    platform_manager: Option<Arc<PlatformManager>>,

    // Runtime plugin state
    active_plugins: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn Plugin>>>>>>,
    plugin_contexts: Arc<RwLock<HashMap<String, PluginContext>>>,
    search_providers: Arc<RwLock<HashMap<String, Arc<dyn SearchProvider>>>>,
    plugin_registry: Arc<RwLock<HashMap<String, PluginManifest>>>,

    // Configuration
    plugins_directory: PathBuf,
}

impl PluginManager {
    /// Create a new plugin manager with configuration
    pub fn new(config: PluginManagerConfig) -> Self {
        let plugins_directory = std::env::var("QORZEN_PLUGINS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("qorzen")
                        .join("plugins")
                }
                #[cfg(target_arch = "wasm32")]
                {
                    PathBuf::from("plugins")
                }
            });

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

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(PluginManagerConfig::default())
    }

    /// Set the event bus manager
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    /// Set the platform manager
    pub fn set_platform_manager(&mut self, platform_manager: Arc<PlatformManager>) {
        self.platform_manager = Some(platform_manager);
    }

    /// Load configuration from file
    #[allow(unused_variables)]
    pub async fn load_config(&mut self, config_path: Option<&str>) -> Result<()> {
        let config_file = config_path.unwrap_or("plugins.toml");

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(content) = tokio::fs::read_to_string(config_file).await {
                if let Ok(config) = toml::from_str::<PluginManagerConfig>(&content) {
                    self.config = config;
                    tracing::info!("Loaded plugin configuration from {}", config_file);
                } else {
                    tracing::warn!("Failed to parse plugin configuration, using defaults");
                }
            } else {
                tracing::info!("No plugin configuration file found, using defaults");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, configuration would typically be loaded via platform manager
            tracing::info!("Using default plugin configuration for WASM environment");
        }

        Ok(())
    }

    /// Register built-in plugins
    pub async fn register_builtin_plugins(&self) -> Result<()> {
        builtin::register_builtin_plugins().await?;
        tracing::info!("Registered built-in plugins");
        Ok(())
    }

    /// Search for plugins in the registry
    pub async fn search_registry(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<RegistryPlugin>> {
        self.registry.search(query, limit).await
    }

    /// Install a plugin from various sources
    pub async fn install_plugin(&self, request: PluginInstallRequest) -> Result<String> {
        tracing::info!("Installing plugin from source: {:?}", request.source);

        match request.source {
            InstallationSource::Local { path } => {
                self.install_from_local(path, request.force_reinstall).await
            }
            InstallationSource::Git { url, branch } => {
                self.install_from_git(url, branch, request.force_reinstall)
                    .await
            }
            InstallationSource::Registry {
                url: _,
                plugin_id,
                version,
            } => {
                self.install_from_registry(plugin_id, version, request.force_reinstall)
                    .await
            }
            InstallationSource::Binary { path } => {
                self.install_from_binary(path, request.force_reinstall)
                    .await
            }
        }
    }

    /// Install plugin from local directory
    #[allow(unused_variables)]
    async fn install_from_local(&self, path: PathBuf, force: bool) -> Result<String> {
        let manifest_path = path.join("plugin.toml");

        #[cfg(not(target_arch = "wasm32"))]
        {
            if !manifest_path.exists() {
                return Err(Error::plugin(
                    "installer",
                    "No plugin.toml found in the specified directory",
                ));
            }

            let manifest = PluginManifest::load_from_file(&manifest_path).await?;
            let plugin_id = manifest.plugin.id.clone();

            // Check if plugin already exists
            let installation_manager = self.installation_manager.lock().await;
            if let Some(_existing) = installation_manager.get_installation(&plugin_id).await {
                if !force {
                    return Err(Error::plugin(
                        &plugin_id,
                        "Plugin already installed. Use force_reinstall to override",
                    ));
                }
                drop(installation_manager);
                self.uninstall_plugin(&plugin_id).await?;
            } else {
                drop(installation_manager);
            }

            // Copy plugin to plugins directory
            let target_dir = self.plugins_directory.join(&plugin_id);
            if target_dir.exists() {
                tokio::fs::remove_dir_all(&target_dir).await.map_err(|e| {
                    Error::plugin(
                        &plugin_id,
                        format!("Failed to remove existing plugin directory: {}", e),
                    )
                })?;
            }

            self.copy_directory(&path, &target_dir).await?;

            // Discover the installation
            let installation_manager = self.installation_manager.lock().await;
            installation_manager.discover_plugins().await?;

            Ok(plugin_id)
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(Error::platform(
                "wasm",
                "plugin_install",
                "Local plugin installation not supported in WASM environment",
            ))
        }
    }

    /// Install plugin from git repository
    #[allow(unused_variables)]
    async fn install_from_git(
        &self,
        url: String,
        branch: Option<String>,
        force: bool,
    ) -> Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let temp_dir = tempfile::tempdir().map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to create temp directory: {}", e),
                )
            })?;

            // Clone the repository
            let mut cmd = tokio::process::Command::new("git");
            cmd.args(["clone"]);

            if let Some(branch) = branch {
                cmd.args(["--branch", &branch]);
            }

            cmd.args([&url, temp_dir.path().to_str().unwrap()]);

            let output = cmd.output().await.map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to clone git repository: {}", e),
                )
            })?;

            if !output.status.success() {
                return Err(Error::plugin(
                    "installer",
                    format!(
                        "Git clone failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }

            // Install from the cloned directory
            self.install_from_local(temp_dir.path().to_path_buf(), force)
                .await
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(Error::platform(
                "wasm",
                "plugin_install",
                "Git plugin installation not supported in WASM environment",
            ))
        }
    }

    /// Install plugin from registry
    #[allow(unused_variables)]
    async fn install_from_registry(
        &self,
        plugin_id: String,
        version: Option<String>,
        _force: bool,
    ) -> Result<String> {
        let _plugin_info = self
            .registry
            .get_plugin(&plugin_id)
            .await?
            .ok_or_else(|| Error::plugin(&plugin_id, "Plugin not found in registry"))?;

        // For now, just simulate installation for WASM
        #[cfg(target_arch = "wasm32")]
        {
            tracing::info!("Simulating plugin installation for WASM: {}", plugin_id);
            return Ok(plugin_id);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // TODO: Implement actual download and installation
            Err(Error::plugin(
                &plugin_id,
                "Registry installation not yet implemented",
            ))
        }
    }

    /// Install plugin from binary
    async fn install_from_binary(&self, _path: PathBuf, _force: bool) -> Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // TODO: Implement binary installation
            Err(Error::plugin(
                "installer",
                "Binary installation not yet implemented",
            ))
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(Error::platform(
                "wasm",
                "plugin_install",
                "Binary plugin installation not supported in WASM environment",
            ))
        }
    }

    /// Load and start a plugin
    pub async fn load_plugin(&self, plugin_id: &str) -> Result<()> {
        tracing::info!("Loading plugin: {}", plugin_id);

        // Check if already loaded
        if self.active_plugins.read().await.contains_key(plugin_id) {
            return Err(Error::plugin(plugin_id, "Plugin already loaded"));
        }

        // Try to create plugin from factory registry first
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

        // Fall back to installation manager
        let installation_manager = self.installation_manager.lock().await;
        let installation = installation_manager
            .get_installation(plugin_id)
            .await
            .ok_or_else(|| Error::plugin(plugin_id, "Plugin not found"))?;

        let context = self.create_plugin_context(&installation.manifest).await?;
        self.plugin_contexts
            .write()
            .await
            .insert(plugin_id.to_string(), context.clone());

        let plugin = installation_manager.load_plugin(plugin_id, context).await?;
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
        Ok(())
    }

    /// Auto-load configured plugins
    pub async fn auto_load_plugins(&self) -> Result<()> {
        if !self.config.auto_load {
            return Ok(());
        }

        // Load default plugins
        for plugin_id in &self.config.default_plugins {
            if let Err(e) = self.load_plugin(plugin_id).await {
                tracing::error!("Failed to auto-load plugin {}: {}", plugin_id, e);
            }
        }

        // Discover and load installed plugins
        let installation_manager = self.installation_manager.lock().await;
        let discovered = installation_manager.discover_plugins().await?;
        drop(installation_manager);

        for plugin_id in discovered {
            if !self.active_plugins.read().await.contains_key(&plugin_id) {
                if let Err(e) = self.load_plugin(&plugin_id).await {
                    tracing::error!("Failed to auto-load discovered plugin {}: {}", plugin_id, e);
                }
            }
        }

        Ok(())
    }

    /// Stop a plugin
    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<()> {
        if let Some(plugin_arc) = self.active_plugins.write().await.remove(plugin_id) {
            let mut plugin = plugin_arc.lock().await;
            plugin.shutdown().await?;

            let installation_manager = self.installation_manager.lock().await;
            installation_manager
                .update_status(plugin_id, PluginStatus::Stopped)
                .await;
            tracing::info!("Plugin {} stopped successfully", plugin_id);
        }
        Ok(())
    }

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        // Stop the plugin if running
        if self.active_plugins.read().await.contains_key(plugin_id) {
            self.stop_plugin(plugin_id).await?;
        }

        // Remove from installation manager
        let installation_manager = self.installation_manager.lock().await;
        installation_manager.uninstall_plugin(plugin_id).await?;

        // Clean up state
        self.plugin_contexts.write().await.remove(plugin_id);
        self.plugin_registry.write().await.remove(plugin_id);

        tracing::info!("Plugin {} uninstalled successfully", plugin_id);
        Ok(())
    }

    /// Render a plugin component
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

    /// Handle API request for a plugin
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

    /// Get plugin statistics
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

    /// Get all UI components from loaded plugins
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

    /// Get all menu items from loaded plugins
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

    /// Create plugin context for a built-in plugin
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

    /// Create plugin context for a loaded plugin
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
                        max_storage_size: Some(100 * 1024 * 1024), // 100MB
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
            config_schema: manifest.settings.clone().unwrap_or(serde_json::json!({})),
            default_values: manifest.settings.clone().unwrap_or(serde_json::json!({})),
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

    /// Copy directory recursively (helper method)
    #[cfg(not(target_arch = "wasm32"))]
    async fn copy_directory(&self, src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
        tokio::fs::create_dir_all(dst).await.map_err(|e| {
            Error::plugin(
                "installer",
                format!("Failed to create target directory: {}", e),
            )
        })?;

        let mut entries = tokio::fs::read_dir(src).await.map_err(|e| {
            Error::plugin(
                "installer",
                format!("Failed to read source directory: {}", e),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            Error::plugin(
                "installer",
                format!("Failed to read directory entry: {}", e),
            )
        })? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if entry
                .file_type()
                .await
                .map_err(|e| Error::plugin("installer", format!("Failed to get file type: {}", e)))?
                .is_dir()
            {
                // Use Box::pin to handle recursive async call
                Box::pin(self.copy_directory(&src_path, &dst_path)).await?;
            } else {
                tokio::fs::copy(&src_path, &dst_path).await.map_err(|e| {
                    Error::plugin("installer", format!("Failed to copy file: {}", e))
                })?;
            }
        }

        Ok(())
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

        // Load configuration
        self.load_config(None).await?;

        // Register built-in plugins
        self.register_builtin_plugins().await?;

        // Initialize installation manager
        self.installation_manager.lock().await.initialize().await?;

        // Auto-load configured plugins
        self.auto_load_plugins().await?;

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
        tracing::info!("Plugin manager shut down");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let config = PluginManagerConfig::default();
        let manager = PluginManager::new(config);
        assert_eq!(manager.name(), "plugin_manager");
    }

    #[tokio::test]
    async fn test_plugin_manager_with_defaults() {
        let manager = PluginManager::with_defaults();
        assert_eq!(manager.config.auto_load, true);
    }

    #[tokio::test]
    async fn test_plugin_stats() {
        let manager = PluginManager::with_defaults();
        let stats = manager.get_plugin_stats().await;
        assert_eq!(stats.total_plugins, 0);
        assert_eq!(stats.active_plugins, 0);
    }
}
